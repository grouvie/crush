use std::net::SocketAddr;

use async_trait::async_trait;
use crush::{
    chrono::Utc,
    rust_ocpp::v1_6::{
        messages::{
            boot_notification::{BootNotificationRequest, BootNotificationResponse},
            heart_beat::{HeartbeatRequest, HeartbeatResponse},
        },
        types::RegistrationStatus,
    },
    Config, CrushBuilder, HandleBootNotificationRequest, HandleHeartbeatRequest, OcppResult,
};
use tracing::{subscriber, Level};
use tracing_subscriber::FmtSubscriber;

struct MyHeartbeatHandler;

#[async_trait]
impl HandleHeartbeatRequest for MyHeartbeatHandler {
    async fn handle(&self, request: HeartbeatRequest) -> OcppResult<HeartbeatResponse> {
        tracing::info!("Handling: {request:#?}");
        let current_time = Utc::now();
        Ok(HeartbeatResponse { current_time })
    }
}

struct MyBootNotificationHandler;

#[async_trait]
impl HandleBootNotificationRequest for MyBootNotificationHandler {
    async fn handle(
        &self,
        request: BootNotificationRequest,
    ) -> OcppResult<BootNotificationResponse> {
        tracing::info!("Handling: {request:#?}");

        let current_time = Utc::now();
        let interval = 30;
        let status = RegistrationStatus::Accepted;

        Ok(BootNotificationResponse {
            current_time,
            interval,
            status,
        })
    }
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();

    subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let address = "127.0.0.1:9100"
        .parse::<SocketAddr>()
        .expect("Parsing SocketAddr failed.");

    let config = Config::new(address);

    let crush = CrushBuilder::new(config)
        .with_heartbeat_handler(MyHeartbeatHandler)
        .with_boot_notification_handler(MyBootNotificationHandler)
        .build();

    if let Err(error) = crush.run().await {
        tracing::error!("Error running Crush: {error}");
    }
}
