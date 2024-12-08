use rust_ocpp::v1_6::messages::start_transaction::{
    StartTransactionRequest, StartTransactionResponse,
};

use std::net::SocketAddr;
use tokio::task::{JoinError, JoinHandle};

pub use chrono;
pub use error::OcppResponseError;
pub use error::OcppResult;
pub use messages::{
    boot_notification::HandleBootNotificationRequest, heartbeat::HandleHeartbeatRequest,
    status_notification::HandleStatusNotificationRequest,
};
pub use rust_ocpp;

mod accept_loop;
mod client_loop;
mod controller_loop;
mod error;
mod messages;
mod serde;
mod server_loop;

use accept_loop::AcceptHandle;
use controller_loop::ControllerHandle;
use server_loop::ServerHandle;

pub trait HandleStartTransactionRequest {
    fn handle(&self, request: StartTransactionRequest) -> StartTransactionResponse;
}

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
    status_notification_handler: Option<Box<dyn HandleStatusNotificationRequest + Send + Sync>>,
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
            status_notification_handler: None,
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
        let controller_handle = ControllerHandle::new(
            self.heartbeat_handler,
            self.boot_notification_handler,
            self.status_notification_handler,
        );

        let (server_handle, server_join) = ServerHandle::new(controller_handle.clone());

        tokio::spawn(async move {
            AcceptHandle::start(self.config.address, server_handle);
        });

        Crush { server_join }
    }
}
