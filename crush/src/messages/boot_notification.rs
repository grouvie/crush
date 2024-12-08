use async_trait::async_trait;
use chrono::Utc;
use rust_ocpp::v1_6::{
    messages::boot_notification::{BootNotificationRequest, BootNotificationResponse},
    types::RegistrationStatus,
};

use crate::error::OcppResult;

#[async_trait]
pub trait HandleBootNotificationRequest: Send + Sync {
    async fn handle(
        &self,
        request: BootNotificationRequest,
    ) -> OcppResult<BootNotificationResponse>;
}

#[derive(Clone)]
pub(crate) struct DefaultBootNotificationHandler;

#[async_trait]
impl HandleBootNotificationRequest for DefaultBootNotificationHandler {
    async fn handle(
        &self,
        _request: BootNotificationRequest,
    ) -> OcppResult<BootNotificationResponse> {
        let current_time = Utc::now();
        let interval = 60;
        let status = RegistrationStatus::Accepted;
        Ok(BootNotificationResponse {
            current_time,
            interval,
            status,
        })
    }
}
