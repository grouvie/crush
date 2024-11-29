use rust_ocpp::v1_6::messages::{
    boot_notification::{BootNotificationRequest, BootNotificationResponse},
    heart_beat::{HeartbeatRequest, HeartbeatResponse},
};
use std::net::SocketAddr;
use tokio::task::{JoinError, JoinHandle};

mod accept_loop;
mod client_loop;
mod controller_loop;
mod error;
mod server_loop;

use accept_loop::AcceptHandle;
pub use chrono;
use controller_loop::ControllerHandle;
pub use rust_ocpp;
use server_loop::ServerHandle;

pub trait HandleHeartbeatRequest {
    fn handle(&self, request: HeartbeatRequest) -> HeartbeatResponse;
}

pub trait HandleBootNotificationRequest {
    fn handle(&self, request: BootNotificationRequest) -> BootNotificationResponse;
}

/*
struct DefaultHeartbeatHandler;

impl HandleHeartbeatRequest for DefaultHeartbeatHandler {
    fn handle(&self, _request: HeartbeatRequest) -> HeartbeatResponse {
        let current_time = Utc::now();
        HeartbeatResponse { current_time }
    }
}

struct DefaultBootNotificationHandler;

impl HandleBootNotificationRequest for DefaultBootNotificationHandler {
    fn handle(&self, _request: BootNotificationRequest) -> BootNotificationResponse {
        let current_time = Utc::now();
        let interval = 60;
        let status = RegistrationStatus::Accepted;
        BootNotificationResponse {
            current_time,
            interval,
            status,
        }
    }
}
*/

#[derive(Clone)]
pub struct Config {
    address: SocketAddr,
}

impl Config {
    #[must_use]
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }
}

pub struct Crush {
    server_join: JoinHandle<()>,
}

impl Crush {
    /// Runs the Crush instance and awaits the completion of the server's join handle.
    ///
    /// # Errors
    ///
    /// This function returns an error of type `JoinError` if the server's task fails to run or is canceled.
    /// The error can occur if the server handle encounters an issue during its execution, such as a panic
    /// or cancellation of the server task.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let crush = Crush::new(config).await.unwrap();
    /// if let Err(e) = crush.run().await {
    ///     eprintln!("Server failed: {}", e);
    /// }
    /// ```
    pub async fn run(self) -> Result<(), JoinError> {
        self.server_join.await
    }
}

pub struct CrushBuilder {
    config: Config,
    heartbeat_handler: Option<Box<dyn HandleHeartbeatRequest + Send + Sync>>,
    boot_notification_handler: Option<Box<dyn HandleBootNotificationRequest + Send + Sync>>,
}

impl CrushBuilder {
    /// Creates a new `CrushBuilder` with the given configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let config = Config::new("127.0.0.1:9100".parse().unwrap());
    /// let builder = CrushBuilder::new(config);
    /// ```
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            heartbeat_handler: None,
            boot_notification_handler: None,
        }
    }

    /// Sets the heartbeat handler.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let config = Config::new("127.0.0.1:9100".parse().unwrap());
    /// let builder = CrushBuilder::new(config).with_heartbeat_handler(MyHeartbeatHandler);
    /// ```
    #[must_use]
    pub fn with_heartbeat_handler<Hr>(mut self, handler: Hr) -> Self
    where
        Hr: HandleHeartbeatRequest + Send + Sync + 'static,
    {
        self.heartbeat_handler = Some(Box::new(handler));
        self
    }

    /// Sets the boot notification handler.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let config = Config::new("127.0.0.1:9100".parse().unwrap());
    /// let builder = CrushBuilder::new(config).with_boot_notification_handler(MyBootNotificationHandler);
    /// ```
    #[must_use]
    pub fn with_boot_notification_handler<Br>(mut self, handler: Br) -> Self
    where
        Br: HandleBootNotificationRequest + Send + Sync + 'static,
    {
        self.boot_notification_handler = Some(Box::new(handler));
        self
    }

    /// Builds a `Crush` instance with the provided configuration.
    ///
    /// # Errors
    ///
    /// This function returns an error of type `std::net::AddrParseError` if the `address` in the configuration
    /// cannot be parsed as a valid `SocketAddr`. This can happen if the address string is incorrectly formatted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let config = Config::new("127.0.0.1:9100".parse().unwrap());
    /// let crush = CrushBuilder::new(config).build();
    /// ```
    #[must_use]
    pub fn build(self) -> Crush {
        let controller_handle =
            ControllerHandle::new(self.heartbeat_handler, self.boot_notification_handler);

        let (server_handle, server_join) = ServerHandle::new(controller_handle.clone());

        tokio::spawn(async move {
            AcceptHandle::start(self.config.address, server_handle);
        });

        Crush { server_join }
    }
}
