use rust_ocpp::v1_6::messages::{
    boot_notification::{BootNotificationRequest, BootNotificationResponse},
    heart_beat::{HeartbeatRequest, HeartbeatResponse},
    status_notification::{StatusNotificationRequest, StatusNotificationResponse},
};
use serde::{de::Error, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Result as JsonResult, Serializer as JsonSerializer, Value};

use crate::OcppResponseError;
pub(crate) struct OcppRequest {
    pub payload: OcppRequestMessage,
    pub uuid: String,
}

#[derive(Debug)]
pub(crate) enum OcppRequestMessage {
    StatusNotification(StatusNotificationRequest),
    BootNotification(BootNotificationRequest),
    Heartbeat(HeartbeatRequest),
}

impl<'de> Deserialize<'de> for OcppRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Vec<Value> = Deserialize::deserialize(deserializer)?;

        if value.len() < 3 {
            return Err(D::Error::custom::<OcppResponseError>(
                OcppResponseError::InvalidRequestFormat {
                    details: Value::String(format!(
                        "Invalid message format: expected at least 3 elements, found {}.",
                        value.len()
                    )),
                },
            ));
        }

        let uuid = value
            .get(1)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                D::Error::custom::<OcppResponseError>(OcppResponseError::InvalidRequestFormat {
                    details: Value::String(
                        "Invalid UUID: expected a string but found none.".to_owned(),
                    ),
                })
            })?
            .to_owned();

        let message_type = value.get(2).and_then(Value::as_str).ok_or_else(|| {
            D::Error::custom::<OcppResponseError>(OcppResponseError::InvalidRequestFormat {
                details: Value::String(
                    "Invalid message type: expected a string but found none.".to_owned(),
                ),
            })
        })?;

        let payload = match message_type {
            "BootNotification" => {
                let payload: BootNotificationRequest =
                    serde_json::from_value(value.get(3).cloned().ok_or_else(|| {
                        D::Error::custom::<OcppResponseError>(
                            OcppResponseError::InvalidRequestFormat {
                                details: Value::String(
                                    "Missing payload for BootNotification.".to_owned(),
                                ),
                            },
                        )
                    })?)
                    .map_err(|error| {
                        D::Error::custom::<OcppResponseError>(
                            OcppResponseError::InvalidRequestFormat {
                                details: Value::String(format!(
                                    "Failed to deserialize BootNotificationRequest: {error}"
                                )),
                            },
                        )
                    })?;
                OcppRequestMessage::BootNotification(payload)
            }
            "Heartbeat" => OcppRequestMessage::Heartbeat(HeartbeatRequest {}),
            "StatusNotification" => {
                let payload: StatusNotificationRequest =
                    serde_json::from_value(value.get(3).cloned().ok_or_else(|| {
                        D::Error::custom::<OcppResponseError>(
                            OcppResponseError::InvalidRequestFormat {
                                details: Value::String(
                                    "Missing payload for StatusNotification.".to_owned(),
                                ),
                            },
                        )
                    })?)
                    .map_err(|error| {
                        D::Error::custom::<OcppResponseError>(
                            OcppResponseError::InvalidRequestFormat {
                                details: Value::String(format!(
                                    "Failed to deserialize StatusNotificationRequest: {error}"
                                )),
                            },
                        )
                    })?;
                OcppRequestMessage::StatusNotification(payload)
            }
            _ => {
                return Err(D::Error::custom::<OcppResponseError>(
                    OcppResponseError::UnsupportedMessageType {
                        details: Value::String(format!("Unknown message type: '{message_type}'.")),
                    },
                ));
            }
        };

        Ok(OcppRequest { payload, uuid })
    }
}

#[derive(Debug, Serialize)]
pub(crate) enum OcppResponseMessage {
    StatusNotification(StatusNotificationResponse),
    BootNotification(BootNotificationResponse),
    Heartbeat(HeartbeatResponse),
    CallError {
        error_code: String,
        error_description: String,
        error_details: Value,
    },
}

impl OcppResponseMessage {
    pub(crate) fn serialize_with_params(
        &self,
        message_type_id: u32,
        uuid: &str,
    ) -> JsonResult<String> {
        let mut json_serializer = JsonSerializer::new(Vec::new());

        match self {
            OcppResponseMessage::BootNotification(payload) => {
                let mut state = json_serializer.serialize_seq(Some(4))?;
                state.serialize_element(&message_type_id)?;
                state.serialize_element(uuid)?;
                state.serialize_element(&"BootNotification")?;
                state.serialize_element(payload)?;
                state.end()?;
            }
            OcppResponseMessage::Heartbeat(payload) => {
                let mut state = json_serializer.serialize_seq(Some(3))?;
                state.serialize_element(&message_type_id)?;
                state.serialize_element(uuid)?;
                state.serialize_element("Heartbeat")?;
                state.serialize_element(payload)?;
                state.end()?;
            }
            OcppResponseMessage::StatusNotification(payload) => {
                let mut state = json_serializer.serialize_seq(Some(3))?;
                state.serialize_element(&message_type_id)?;
                state.serialize_element(uuid)?;
                state.serialize_element("StatusNotification")?;
                state.serialize_element(payload)?;
                state.end()?;
            }
            OcppResponseMessage::CallError {
                error_code,
                error_description,
                error_details,
            } => {
                let mut state = json_serializer.serialize_seq(Some(5))?;
                state.serialize_element(&4)?;
                state.serialize_element(uuid)?;
                state.serialize_element(error_code)?;
                state.serialize_element(error_description)?;
                state.serialize_element(error_details)?;
                state.end()?;
            }
        }

        let json_string = String::from_utf8(json_serializer.into_inner()).map_err(|error| {
            serde_json::Error::custom(format!("Failed to convert to String: {error}"))
        })?;
        Ok(json_string)
    }
}
