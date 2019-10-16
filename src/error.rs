
use custom_error::custom_error;

custom_error!{pub WebmetroError
    ResourcesExceeded = "resources exceeded",
    EbmlError{source: crate::ebml::EbmlError} = "EBML error: {source}",
    HttpError{source: http::Error} = "HTTP error: {source}",
    HyperError{source: hyper::Error} = "Hyper error: {source}",
    IoError{source: std::io::Error} = "IO error: {source}",
    TimerError{source: tokio::timer::Error} = "Timer error: {source}",
    WarpError{source: warp::Error} = "Warp error: {source}",
    ApplicationError{message: String} = "{message}"
}

impl From<&str> for WebmetroError {
    fn from(message: &str) -> WebmetroError {
        WebmetroError::ApplicationError{message: message.into()}
    }
}
