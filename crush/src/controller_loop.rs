use rust_ocpp::v1_6::messages::{
    boot_notification::BootNotificationRequest, heart_beat::HeartbeatRequest,
};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

use crate::{error::CrushResult, HandleBootNotificationRequest, HandleHeartbeatRequest};

pub(crate) enum ToController {
    Message(String, oneshot::Sender<String>),
}

struct Controller {
    receiver: Receiver<ToController>,
    heartbeat_handler: Option<Box<dyn HandleHeartbeatRequest + Send + Sync>>,
    boot_notification_handler: Option<Box<dyn HandleBootNotificationRequest + Send + Sync>>,
}

impl Controller {
    fn new(
        receiver: Receiver<ToController>,
        heartbeat_handler: Option<Box<dyn HandleHeartbeatRequest + Send + Sync>>,
        boot_notification_handler: Option<Box<dyn HandleBootNotificationRequest + Send + Sync>>,
    ) -> Self {
        Self {
            receiver,
            heartbeat_handler,
            boot_notification_handler,
        }
    }

    fn handle_message(&self, msg: ToController) {
        match msg {
            ToController::Message(message, sender) => {
                tracing::info!("{message}");
                // Use the heartbeat handler

                /*
                I want to use one handler for each ocpp message in here
                HeartBeat,
                BootNotification,
                Authorize,
                MeterValues
                StartTransaction
                ...
                 */

                let heartbeat_request = HeartbeatRequest {};
                if let Some(heartbeat_handler) = &self.heartbeat_handler {
                    let heartbeat_response = heartbeat_handler.handle(heartbeat_request);
                    tracing::info!("{heartbeat_response:?}");
                } else {
                    tracing::info!("Heartbeat Unsupported");
                }
                let boot_notification_request = BootNotificationRequest {
                    charge_box_serial_number: None,
                    charge_point_model: "Grouvie Model".to_owned(),
                    charge_point_serial_number: None,
                    charge_point_vendor: "Grouvie Vendor".to_owned(),
                    firmware_version: None,
                    iccid: None,
                    imsi: None,
                    meter_serial_number: None,
                    meter_type: None,
                };
                if let Some(boot_notification_handler) = &self.boot_notification_handler {
                    let boot_notification_response =
                        boot_notification_handler.handle(boot_notification_request);
                    tracing::info!("{boot_notification_response:?}");
                } else {
                    tracing::info!("BootNotification Unsupported");
                }
                // Additional logic for boot_notification and other handlers can go here
                drop(sender.send("Response".to_owned()));
            }
        }
    }
}

async fn run_controller(mut controller_actor: Controller) -> CrushResult<()> {
    while let Some(msg) = controller_actor.receiver.recv().await {
        controller_actor.handle_message(msg);
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) struct ControllerHandle {
    sender: Sender<ToController>,
}

impl ControllerHandle {
    pub(crate) fn new(
        heartbeat_handler: Option<Box<dyn HandleHeartbeatRequest + Send + Sync>>,
        boot_notification_handler: Option<Box<dyn HandleBootNotificationRequest + Send + Sync>>,
    ) -> Self {
        let (sender, receiver) = channel(64);

        tokio::spawn(async move {
            let actor = Controller::new(receiver, heartbeat_handler, boot_notification_handler);
            if let Err(error) = run_controller(actor).await {
                tracing::error!("{error}");
            };
        });

        Self { sender }
    }
    pub(crate) async fn send(&mut self, msg: ToController) {
        assert!(
            self.sender.send(msg).await.is_ok(),
            "Controller loop has shut down"
        );
    }
}
