# cliff [[!GitHub actions](https://github.com/meetup/cliff/workflows/Main/badge.svg)](https://github.com/meetup/cliff/actions) [![Coverage Status](https://coveralls.io/repos/github/meetup/cliff/badge.svg?branch=master)](https://coveralls.io/github/meetup/cliff?branch=master)

> An AWS CloudFormation stack diff tool

## 🥰 features

* 🏗️visible and predictable infrastructure deployment outcomes
* 👩‍💻optimized for human DX

## 🤔 about

[CloudFormation](https://aws.amazon.com/cloudformation/) is an awesome fully managed infrastructure as code tool but applying these changes to your existing infrastructure can sometimes have suprising effects. Cliff can help with that.

Cliff is enhances CloudFormation by enabling you to see your changes will look like before you make them  💅 . You can think of cliff as the combination of create changeset + describe changeset + diff packaged into one tool. You _can_ do all of these things without cliff using the aws cli the result will come at the expense of learning to cross stitch and understanding nuances of CloudFormation details 🧶. Cliff does that stitching for you with a display intended for humans.

## 🤸 usage

Basic usage requires two things. The name of an existing stack and a path to a template

```sh
$ cliff \
	--stack-name your-cloud-formation-stack-name \
	path/to/template.yml
```

💡Cliff will report when there is nothing in the changset between the current stack and the remote stack.

Many CloudFormation templates will employ parameterization for flexibility. By default, cliff will reuse the parameters previously provided but often you'll want to change those for new deployments. In those cases, use the `--parameters` or `-p` option.

```sh
$ cliff \
	--stack-name your-cloud-formation-stack-name \
	--parameters "Foo=bar" "Baz=boom" "quux=quuz" \
	path/to/template.yml
```

### diffing

By default cliff will `diff --label -u` to compare local and remote templates. If you would like a fancier diff tool, cliff will use the value of 
an environment variable `CLIFF_DIFFER` instead. 

#### vscode 
If you are a [VS Code](https://code.visualstudio.com/) user you may want to use `CLIFF_DIFFER="code --wait --diff"`

#### colordiff

If you are a [colordiff](https://www.colordiff.org/) user you may want to use use `CLIFF_DIFFER=colordiff`

### changesets

Cliff leverages the CloudFormation feature of creating a remote changeset to understand and predict the effect of a CloudFormation deployment. 
In particular you may be interested in what will be added, removed, or modified in you stack before actually deploying your changes.

## 👩‍🏭 development

This is a [rustlang](https://www.rust-lang.org/en-US/) application.
Go grab yourself a copy with [rustup](https://rustup.rs/)


Meetup Inc 2019