use rusoto_cloudformation::{
    CreateChangeSetError, DeleteChangeSetError, DescribeChangeSetError, DescribeStacksError,
    GetTemplateError,
};
use rusoto_core::{request::BufferedHttpResponse, RusotoError};
use serde::Deserialize;
use std::{error::Error as StdError, fmt};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ErrorResponse {
    error: ErrorResponseError,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ErrorResponseError {
    code: String,
    message: String,
}

/// possible error cases
#[derive(Debug, PartialEq)]
pub enum Error {
    Get(RusotoError<GetTemplateError>),
    Create(RusotoError<CreateChangeSetError>),
    DescribeChangeset(RusotoError<DescribeChangeSetError>),
    DescribeStack(RusotoError<DescribeStacksError>),
    Delete(RusotoError<DeleteChangeSetError>),
    Differ(String),
    Validation(String),
    Throttling(String),
}

impl From<RusotoError<GetTemplateError>> for Error {
    fn from(err: RusotoError<GetTemplateError>) -> Self {
        match &err {
            // deal with the fact that Rusoto doesn't suface structured errors well here
            RusotoError::Unknown(BufferedHttpResponse { ref body, .. }) => {
                if let Ok(ErrorResponse { error }) =
                    serde_xml_rs::from_reader::<_, ErrorResponse>(body.as_ref())
                {
                    match error.code.as_str() {
                        "Throttling" => return Error::Throttling(error.message),
                        "ValidationError" => return Error::Validation(error.message),
                        code => log::debug!("unmatched error code {}", code),
                    }
                }
                Error::Get(err)
            }
            _ => Error::Get(err),
        }
    }
}

impl From<RusotoError<CreateChangeSetError>> for Error {
    fn from(err: RusotoError<CreateChangeSetError>) -> Self {
        match &err {
            // deal with the fact that Rusoto doesn't suface structured errors well here
            RusotoError::Unknown(BufferedHttpResponse { ref body, .. }) => {
                if let Ok(ErrorResponse { error }) =
                    serde_xml_rs::from_reader::<_, ErrorResponse>(body.as_ref())
                {
                    match error.code.as_str() {
                        "ValidationError" => return Error::Validation(error.message),
                        "Throttling" => return Error::Throttling(error.message),
                        code => log::debug!("unmatched error code {}", code),
                    }
                }
                Error::Create(err)
            }
            _ => Error::Create(err),
        }
    }
}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Error::Get(e) => e.to_string(),
                Error::Create(e) => e.to_string(),
                Error::DescribeChangeset(e) => e.to_string(),
                Error::DescribeStack(e) => e.to_string(),
                Error::Delete(e) => e.to_string(),
                Error::Differ(tool) => format!("Invalid differ tool {}", tool),
                Error::Validation(message) => format!("Error: {}", message),
                Error::Throttling(message) => message.to_string(),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn error_response_deserializes() {
        assert!(
					serde_xml_rs::from_str::<ErrorResponse>(
						"<ErrorResponse><Error><Code>ValidationError</Code><Message>test</Message></Error></ErrorResponse>"
					).is_ok()
			 );
    }

    #[test]
    fn error_from_create_changset_error() -> Result<(), Box<dyn StdError>> {
        let rusoto_error: RusotoError<CreateChangeSetError> =
            RusotoError::Service(CreateChangeSetError::AlreadyExists("test".into()));
        let err = Error::from(rusoto_error);
        assert_eq!(
            err,
            Error::Create(RusotoError::Service(CreateChangeSetError::AlreadyExists(
                "test".into()
            )))
        );
        Ok(())
    }

    #[test]
    fn error_from_create_changset_error_validation() -> Result<(), Box<dyn StdError>> {
        let rusoto_error: RusotoError<CreateChangeSetError> =
            RusotoError::Unknown(BufferedHttpResponse {
                status: Default::default(),
                body: Bytes::from("<ErrorResponse><Error><Code>ValidationError</Code><Message>test</Message></Error></ErrorResponse>"),
                headers: Default::default(),
            });
        let err = Error::from(rusoto_error);
        assert_eq!(err, Error::Validation("test".into()));
        Ok(())
    }

    #[test]
    fn error_from_create_changset_error_throttling() -> Result<(), Box<dyn StdError>> {
        let rusoto_error: RusotoError<CreateChangeSetError> =
            RusotoError::Unknown(BufferedHttpResponse {
                status: Default::default(),
                body: Bytes::from("<ErrorResponse><Error><Code>Throttling</Code><Message>test</Message></Error></ErrorResponse>"),
                headers: Default::default(),
            });
        let err = Error::from(rusoto_error);
        assert_eq!(err, Error::Throttling("test".into()));
        Ok(())
    }

    #[test]
    fn error_from_get_template_error() -> Result<(), Box<dyn StdError>> {
        let rusoto_error: RusotoError<GetTemplateError> =
            RusotoError::Service(GetTemplateError::ChangeSetNotFound("test".into()));
        let err = Error::from(rusoto_error);
        assert_eq!(
            err,
            Error::Get(RusotoError::Service(GetTemplateError::ChangeSetNotFound(
                "test".into()
            )))
        );
        Ok(())
    }

    #[test]
    fn error_from_get_template_error_validation() -> Result<(), Box<dyn StdError>> {
        let rusoto_error: RusotoError<GetTemplateError> =
            RusotoError::Unknown(BufferedHttpResponse {
                status: Default::default(),
                body: Bytes::from("<ErrorResponse><Error><Code>ValidationError</Code><Message>test</Message></Error></ErrorResponse>"),
                headers: Default::default(),
            });
        let err = Error::from(rusoto_error);
        assert_eq!(err, Error::Validation("test".into()));
        Ok(())
    }

    #[test]
    fn error_from_get_template_error_throttling() -> Result<(), Box<dyn StdError>> {
        let rusoto_error: RusotoError<GetTemplateError> =
            RusotoError::Unknown(BufferedHttpResponse {
                status: Default::default(),
                body: Bytes::from("<ErrorResponse><Error><Code>Throttling</Code><Message>test</Message></Error></ErrorResponse>"),
                headers: Default::default(),
            });
        let err = Error::from(rusoto_error);
        assert_eq!(err, Error::Throttling("test".into()));
        Ok(())
    }
}
