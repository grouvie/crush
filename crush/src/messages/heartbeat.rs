use async_trait::async_trait;
use chrono::Utc;
use rust_ocpp::v1_6::messages::heart_beat::{HeartbeatRequest, HeartbeatResponse};

use crate::error::OcppResult;

#[async_trait]
pub trait HandleHeartbeatRequest: Send + Sync {
    async fn handle(&self, request: HeartbeatRequest) -> OcppResult<HeartbeatResponse>;
}

pub(crate) struct DefaultHeartbeatHandler;

#[async_trait]
impl HandleHeartbeatRequest for DefaultHeartbeatHandler {
    async fn handle(&self, _request: HeartbeatRequest) -> OcppResult<HeartbeatResponse> {
        let current_time = Utc::now();
        Ok(HeartbeatResponse { current_time })
    }
}
