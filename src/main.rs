//! An AWS CloudFormation stack diff tool
use colored::Colorize;
use futures::{future, Future};
use futures_backoff::Strategy;
use lazy_static::lazy_static;
use rusoto_cloudformation::{
    Change, CloudFormation, CloudFormationClient, CreateChangeSetError, CreateChangeSetInput,
    CreateChangeSetOutput, DeleteChangeSetError, DeleteChangeSetInput, DescribeChangeSetError,
    DescribeChangeSetInput, DescribeChangeSetOutput, GetTemplateInput, GetTemplateOutput,
    Parameter,
};
use rusoto_core::{credential::ChainProvider, request::HttpClient, Region, RusotoError};
use std::{
    env,
    error::Error as StdError,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{exit, Command},
    str::{from_utf8, FromStr},
    thread::sleep,
    time::Duration,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

mod error;
use crate::error::Error;

const CHANGESET_NAME: &str = "cliff";

lazy_static! {
    static ref RETRIES: Strategy = Strategy::exponential(Duration::from_millis(100))
        .with_max_retries(15)
        .with_jitter(true);
}

fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn StdError>>
where
    T: FromStr,
    T::Err: StdError + 'static,
    U: FromStr,
    U::Err: StdError + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

#[derive(Debug, StructOpt)]
#[structopt(name = "cliff", about = "A CloudFormation stack diff tool")]
struct Options {
    #[structopt(
        short = "p",
        long = "parameters",
        parse(try_from_str = "parse_key_val"),
        help = "multi-valued parameter for providing template parameters in the form 'parameter-name=parameter-value'"
    )]
    parameters: Vec<(String, String)>,
    #[structopt(
        short = "s",
        long = "stack-name",
        help = "name of the CloudFormation stack to diff against"
    )]
    stack_name: String,
    #[structopt(help = "filename of local template")]
    filename: PathBuf,
}

fn credentials() -> ChainProvider {
    let mut chain = ChainProvider::new();
    chain.set_timeout(Duration::from_millis(200));
    chain
}

fn client() -> CloudFormationClient {
    CloudFormationClient::new_with(
        HttpClient::new().expect("failed to create request dispatcher"),
        credentials(),
        Region::default(),
    )
}

fn current_template(
    cf: CloudFormationClient,
    stack_name: String,
) -> impl Future<Item = GetTemplateOutput, Error = Error> {
    RETRIES.retry_if(
        move || {
            cf.get_template(GetTemplateInput {
                stack_name: Some(stack_name.clone()),
                template_stage: Some("Original".into()),
                ..GetTemplateInput::default()
            })
            .map_err(Error::from)
        },
        |err: &Error| {
            log::debug!("get template error {}", err);
            match err {
                Error::Throttling(_) => true,
                _ => false,
            }
        },
    )
}

fn create_changeset(
    cf: CloudFormationClient,
    stack_name: String,
    template_body: String,
    parameters: Vec<(String, String)>,
) -> impl Future<Item = CreateChangeSetOutput, Error = Error> {
    RETRIES.retry_if(
        move || {
            cf.create_change_set(CreateChangeSetInput {
                change_set_name: CHANGESET_NAME.into(),
                stack_name: stack_name.clone(),
                template_body: Some(template_body.clone()),
                capabilities: Some(vec!["CAPABILITY_IAM".into(), "CAPABILITY_NAMED_IAM".into()]),
                parameters: Some(
                    parameters
                        .clone()
                        .into_iter()
                        .map(|(k, v)| Parameter {
                            parameter_key: Some(k),
                            parameter_value: Some(v),
                            ..Parameter::default()
                        })
                        .collect(),
                ),
                ..CreateChangeSetInput::default()
            })
            .map_err(Error::from)
        },
        move |err: &Error| {
            log::debug!("create changeset error {}", err);
            match err {
                Error::Create(RusotoError::Service(CreateChangeSetError::LimitExceeded(_))) => true,
                Error::Throttling(_) => true,
                _ => false,
            }
        },
    )
}

fn describe_changeset(
    cf: CloudFormationClient,
    stack_name: String,
) -> Box<
    dyn Future<Item = DescribeChangeSetOutput, Error = RusotoError<DescribeChangeSetError>> + Send,
> {
    Box::new(
        cf.clone()
            .describe_change_set(DescribeChangeSetInput {
                change_set_name: CHANGESET_NAME.into(),
                stack_name: Some(stack_name.clone()),
                ..DescribeChangeSetInput::default()
            })
            .and_then(move |response| {
                if response
                    .status
                    .iter()
                    .any(|v| v.ends_with("_PROGRESS") || v.ends_with("_PENDING"))
                {
                    sleep(Duration::from_millis(500));
                    future::Either::A(describe_changeset(cf, stack_name))
                } else {
                    future::Either::B(future::ok(response))
                }
            }),
    )
}

fn delete_changset(
    cf: CloudFormationClient,
    stack_name: String,
) -> impl Future<Item = (), Error = RusotoError<DeleteChangeSetError>> {
    cf.delete_change_set(DeleteChangeSetInput {
        change_set_name: CHANGESET_NAME.into(),
        stack_name: Some(stack_name),
    })
    .map(drop)
}

fn render_change(change: Change) -> String {
    let c = change.resource_change.unwrap_or_default();

    let line = format!(
        "{} {} {} {} {} {}",
        c.action.clone().unwrap_or_default().bold(),
        c.resource_type.unwrap_or_default().dimmed(),
        c.logical_resource_id.unwrap_or_default().bold(),
        c.physical_resource_id.unwrap_or_default().dimmed(),
        c.scope.unwrap_or_default().join(", ").bold(),
        if c.replacement.unwrap_or_default() == "True" {
            " ⚠️  Requires replacement"
        } else {
            ""
        },
    );
    match c.action.unwrap_or_default().as_str() {
        "Modify" => format!("🔧 {}", line.bright_yellow()),
        "Remove" => format!("✂️  {}", line.bright_red()),
        "Add" => format!("🌱 {}", line.bright_green()),
        _ => line,
    }
}

fn diff_changeset(changeset: DescribeChangeSetOutput) {
    if changeset.status.iter().any(|v| v.ends_with("_COMPLETE")) {
        let mut changes = changeset.changes.unwrap_or_default();
        changes.sort_by(|a, b| {
            a.resource_change
                .clone()
                .unwrap_or_default()
                .action
                .unwrap_or_default()
                .cmp(
                    &b.resource_change
                        .clone()
                        .unwrap_or_default()
                        .action
                        .unwrap_or_default(),
                )
        });
        for change in changes {
            if change.type_.clone().unwrap_or_default() == "Resource" {
                println!("{}", render_change(change));
            } else {
                println!("other {:#?}", change);
            }
        }
    } else {
        println!(
            "change set status is {}",
            changeset.status.unwrap_or_default()
        );
    }
}

fn suffix_tempfile(filename: &PathBuf) -> io::Result<tempfile::NamedTempFile> {
    Ok(tempfile::Builder::new()
        .suffix(
            &filename
                .extension()
                .map(|x| format!(".{}", x.to_str().unwrap_or_default()))
                .unwrap_or_default(),
        )
        .tempfile()?)
}

fn diff_template(
    filename: &PathBuf,
    template_body: String,
) -> Result<String, Box<dyn StdError>> {
    let mut tmp = suffix_tempfile(&filename)?;
    tmp.write_all(&template_body.as_bytes().to_vec()[..])?;
    tmp.flush()?;
    let path = tmp.path().to_str().unwrap_or_default();
    let tool = env::var("CLIFF_DIFFER")
        .ok()
        .unwrap_or_else(|| "diff --label -u".to_string());
    let elements = tool.split_whitespace().collect::<Vec<_>>();
    let (program, args) = match elements.split_first() {
        Some(pair) => pair,
        _ => return Err(Box::new(Error::Differ(tool))),
    };
    let output = args
        .iter()
        .fold(&mut Command::new(program), |cmd, arg| cmd.arg(arg))
        .args(&[filename.to_str().unwrap_or_default(), path])
        .output()?;
    // if output.status.code().unwrap_or_default() != 0 {
    //     eprintln!("{}", from_utf8(&output.stderr)?);
    //     return Err(Box::new(Error::Differ(tool)));
    // }
    Ok(from_utf8(&output.stdout)?.into())
}

fn template_body<P: AsRef<Path>>(filename: P) -> io::Result<String> {
    fs::read_to_string(filename)
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        exit(1)
    }
}

fn run() -> Result<(), Box<dyn StdError>> {
    env_logger::init();
    let Options {
        parameters,
        stack_name,
        filename,
    } = Options::from_args();
    let stack_name2 = stack_name.clone();
    let cf = client();

    let current_template = current_template(cf.clone(), stack_name.clone());
    let body = template_body(filename.clone())?;
    let changeset = create_changeset(cf.clone(), stack_name.clone(), body, parameters);

    let diff_templates = current_template.and_then(move |current| {
        match diff_template(&filename, current.template_body.unwrap_or_default()) {
            Ok(diff) => {
                println!("{}", diff);
                Ok(())
            },
            _/*todo*/ => Ok(())
        }
    });

    let stack_name3 = stack_name.clone();
    let cf2 = cf.clone();
    let diff_changeset = diff_templates.and_then(|_| changeset).and_then(move |_| {
        describe_changeset(cf.clone(), stack_name3)
            .map_err(Error::Describe)
            .map(diff_changeset)
    });

    let complete =
        diff_changeset.and_then(move |_| delete_changset(cf2, stack_name2).map_err(Error::Delete));

    Runtime::new().unwrap().block_on(complete)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn template_body_reads_from_disk() {
        assert!(template_body("tests/data/template-after.yml").is_ok())
    }

    #[test]
    fn diff_template_yields_diff() -> Result<(), Box<dyn StdError>> {
        let diff = diff_template(
            &PathBuf::from("tests/data/template-before.yml"),
            include_str!("../tests/data/template-after.yml").into(),
        )?;
        assert_eq!(
            diff,
            r#"5c5
<       TableName: test
\ No newline at end of file
---
>       TableName: test2
\ No newline at end of file
"#
        );
        Ok(())
    }
}
