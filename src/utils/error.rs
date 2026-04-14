use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatrixError {
    pub errcode: String,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpError {
    pub message: String,
    pub status: u16,
    pub body: Option<MatrixError>,
    pub request_id: Option<String>,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref rid) = self.request_id {
            write!(f, "{} (ref: {})", self.message, rid)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for HttpError {}

pub fn display_error(errcode: &str, status: u16, message: &str) -> String {
    format!("{errcode} ({status}): {message}")
}

/// Format an error message for display in toasts, including the request ID if available.
pub fn format_error_with_ref(error: &HttpError) -> String {
    if let Some(ref rid) = error.request_id {
        format!("{} (ref: {})", error.message, rid)
    } else {
        error.message.clone()
    }
}
