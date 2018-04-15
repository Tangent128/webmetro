use std::{
    error::Error,
    fmt::{
        Display,
        Formatter,
        Result as FmtResult
    },
    io::Error as IoError
};

use ebml::EbmlError;

#[derive(Debug)]
pub enum WebmetroError {
    EbmlError(EbmlError),
    IoError(IoError),
    Unknown(Box<Error + Send>)
}

impl WebmetroError {
    pub fn from_str(string: &str) -> WebmetroError {
        string.into()
    }

    pub fn from_err<E: Error + Send + 'static>(err: E) -> WebmetroError {
        WebmetroError::Unknown(Box::new(err))
    }
}

impl Display for WebmetroError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            &WebmetroError::EbmlError(ref err) => err.fmt(f),
            &WebmetroError::IoError(ref err) => err.fmt(f),
            &WebmetroError::Unknown(ref err) => err.fmt(f),
        }
    }
}
impl Error for WebmetroError {
    fn description(&self) -> &str {
        match self {
            &WebmetroError::EbmlError(ref err) => err.description(),
            &WebmetroError::IoError(ref err) => err.description(),
            &WebmetroError::Unknown(ref err) => err.description(),
        }
    }
}

impl From<EbmlError> for WebmetroError {
    fn from(err: EbmlError) -> WebmetroError {
        WebmetroError::EbmlError(err)
    }
}

impl From<IoError> for WebmetroError {
    fn from(err: IoError) -> WebmetroError {
        WebmetroError::IoError(err)
    }
}

impl From<Box<Error + Send>> for WebmetroError {
    fn from(err: Box<Error + Send>) -> WebmetroError {
        WebmetroError::Unknown(err)
    }
}

impl<'a> From<&'a str> for WebmetroError {
    fn from(err: &'a str) -> WebmetroError {
        let error: Box<Error + Send + Sync> = err.into();
        WebmetroError::Unknown(error)
    }
}
