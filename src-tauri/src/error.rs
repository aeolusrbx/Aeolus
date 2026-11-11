//! Unified error type returned to the frontend.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Network error: {0}")]
    Network(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Another operation is already running: {0}")]
    Busy(String),
    #[error("Integrity check failed: {0}")]
    Integrity(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => "permission",
            AppError::Io(_) => "io",
            AppError::Json(_) => "data",
            AppError::Network(_) => "network",
            AppError::Validation(_) => "validation",
            AppError::NotFound(_) => "not_found",
            AppError::Cancelled => "cancelled",
            AppError::Busy(_) => "busy",
            AppError::Integrity(_) => "integrity",
            AppError::Unsupported(_) => "unsupported",
            AppError::Other(_) => "other",
        }
    }

    /// A short, user-facing hint describing how to recover.
    pub fn hint(&self) -> Option<&'static str> {
        match self.kind() {
            "permission" => Some("Aeolus was denied access. Close Roblox if it is running, and make sure the folder is not read-only or protected."),
            "network" => Some("Check your internet connection or firewall, then try again. Partially downloaded files are kept and resumed."),
            "integrity" => Some("A downloaded or installed file did not match the expected checksum. Run Repair to re-download it."),
            "busy" => Some("Wait for the current operation to finish or cancel it."),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::Integrity(format!("could not read package archive: {e}"))
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 3)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.serialize_field("hint", &self.hint())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
