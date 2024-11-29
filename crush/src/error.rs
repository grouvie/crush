use hyper_tungstenite::tungstenite::{self, http};
use std::io;
use tokio::sync::oneshot;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CrushError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    HyperHttp(#[from] http::Error),

    #[error(transparent)]
    OneshotReceive(#[from] oneshot::error::RecvError),

    #[error(transparent)]
    Tungstenite(#[from] tungstenite::Error),
}
/*
impl fmt::Display for CrushError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenericError(msg) => write!(formatter, "GenericError: {msg}"),
        }
    }
}
*/

pub(crate) type CrushResult<T, E = CrushError> = Result<T, E>;
