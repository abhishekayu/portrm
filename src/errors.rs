use thiserror::Error;

#[derive(Debug, Error)]
pub enum PtrmError {
    #[error("no process found on port {0}")]
    NoProcessOnPort(u16),

    #[error("permission denied: killing PID {pid} requires elevated privileges")]
    PermissionDenied { pid: u32 },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, PtrmError>;
