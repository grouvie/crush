use async_trait::async_trait;
use rust_ocpp::v1_6::messages::status_notification::{
    StatusNotificationRequest, StatusNotificationResponse,
};

use crate::error::OcppResult;

#[async_trait]
pub trait HandleStatusNotificationRequest: Send + Sync {
    async fn handle(
        &self,
        request: StatusNotificationRequest,
    ) -> OcppResult<StatusNotificationResponse>;
}
pub(crate) struct DefaultStatusNotificationHandler;

#[async_trait]
impl HandleStatusNotificationRequest for DefaultStatusNotificationHandler {
    async fn handle(
        &self,
        _request: StatusNotificationRequest,
    ) -> OcppResult<StatusNotificationResponse> {
        Ok(StatusNotificationResponse {})
    }
}
