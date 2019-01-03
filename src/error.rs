
use custom_error::custom_error;

custom_error!{pub WebmetroError
    ResourcesExceeded = "resources exceeded",
    EbmlError{source: crate::ebml::EbmlError} = "EBML error",
    HttpError{source: http::Error} = "HTTP error",
    HyperError{source: hyper::Error} = "Hyper error",
    IoError{source: std::io::Error} = "IO error",
    TimerError{source: tokio::timer::Error} = "Timer error",
    WarpError{source: warp::Error} = "Warp error",
    ApplicationError{message: String} = "{message}"
}

impl From<&str> for WebmetroError {
    fn from(message: &str) -> WebmetroError {
        WebmetroError::ApplicationError{message: message.into()}
    }
}
