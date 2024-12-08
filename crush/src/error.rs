use hyper_tungstenite::tungstenite::{self, http};
use serde_json::Value;
use std::io;
use tokio::sync::oneshot;

use crate::serde::OcppResponseMessage;

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

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
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

#[derive(Debug, thiserror::Error)]
pub enum OcppResponseError {
    #[error("Generic error")]
    Generic,

    #[error("Invalid request format")]
    InvalidRequestFormat { details: Value },

    #[error("Unsupported message type")]
    UnsupportedMessageType { details: Value },

    #[error("Internal server error")]
    InternalError,
}

pub type OcppResult<T> = Result<T, OcppResponseError>;

pub(crate) trait IntoOcppRequestMessage {
    fn into_ocpp_response(self) -> OcppResponseMessage;
}

impl IntoOcppRequestMessage for OcppResponseError {
    fn into_ocpp_response(self) -> OcppResponseMessage {
        match self {
            Self::InvalidRequestFormat { details} => OcppResponseMessage::CallError {
                error_code: "FormationViolation".to_owned(),
                error_description: "Payload for Action is syntactically incorrect or not conform the PDU structure for Action".to_owned(),
                error_details: details,
            },
            Self::UnsupportedMessageType { details}=> OcppResponseMessage::CallError {
                error_code: "NotSupported".to_owned(),
                error_description: "Requested Action is recognized but not supported by the receiver".to_owned(),
                error_details: details,
            },
            Self::InternalError => OcppResponseMessage::CallError {
                error_code: "InternalError".to_owned(),
                error_description: "An internal error occurred and the receiver was not able to process the requested Action successfully".to_owned(),
                error_details: Value::default(),
            },
            Self::Generic => OcppResponseMessage::CallError {
                error_code: "GenericError".to_owned(),
                error_description: "Something unexpected happened.".to_owned(),
                error_details: Value::default(),
            },
        }
    }
}
