use crate::{
    error::{CrushResult, IntoOcppRequestMessage},
    messages::{
        boot_notification::DefaultBootNotificationHandler, heartbeat::DefaultHeartbeatHandler,
        status_notification::DefaultStatusNotificationHandler,
    },
    serde::{OcppRequest, OcppRequestMessage, OcppResponseMessage},
    HandleBootNotificationRequest, HandleHeartbeatRequest, HandleStatusNotificationRequest,
};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

pub(crate) enum ToController {
    Message(String, oneshot::Sender<String>),
}

struct Controller {
    receiver: Receiver<ToController>,
    heartbeat_handler: Option<Box<dyn HandleHeartbeatRequest + Send + Sync>>,
    boot_notification_handler: Option<Box<dyn HandleBootNotificationRequest + Send + Sync>>,
    status_notification_handler: Option<Box<dyn HandleStatusNotificationRequest + Send + Sync>>,
}

impl Controller {
    fn new(
        receiver: Receiver<ToController>,
        heartbeat_handler: Option<Box<dyn HandleHeartbeatRequest + Send + Sync>>,
        boot_notification_handler: Option<Box<dyn HandleBootNotificationRequest + Send + Sync>>,
        status_notification_handler: Option<Box<dyn HandleStatusNotificationRequest + Send + Sync>>,
    ) -> Self {
        Self {
            receiver,
            heartbeat_handler,
            boot_notification_handler,
            status_notification_handler,
        }
    }
    async fn handle_message(&self, msg: ToController) -> CrushResult<()> {
        match msg {
            ToController::Message(message, sender) => {
                let ocpp_request = match serde_json::from_str::<OcppRequest>(&message) {
                    Ok(ocpp_request) => ocpp_request,
                    Err(error) => {
                        tracing::error!(
                            "Failed to deserialize OcppRequest: {error}. \n Input: \n {message}"
                        );
                        return Ok(());
                    }
                };

                let uuid = ocpp_request.uuid;
                let payload = ocpp_request.payload;

                let ocpp_response_message = self.process(payload).await;

                let response = ocpp_response_message.serialize_with_params(3, &uuid)?;
                drop(sender.send(response));
            }
        }
        Ok(())
    }
    async fn process(&self, msg: OcppRequestMessage) -> OcppResponseMessage {
        match msg {
            OcppRequestMessage::StatusNotification(request) => {
                if let Some(handler) = &self.status_notification_handler {
                    match handler.handle(request).await {
                        Ok(response) => OcppResponseMessage::StatusNotification(response),
                        Err(error) => error.into_ocpp_response(),
                    }
                } else {
                    match DefaultStatusNotificationHandler.handle(request).await {
                        Ok(response) => OcppResponseMessage::StatusNotification(response),
                        Err(error) => error.into_ocpp_response(),
                    }
                }
            }
            OcppRequestMessage::BootNotification(request) => {
                if let Some(handler) = &self.boot_notification_handler {
                    match handler.handle(request).await {
                        Ok(response) => OcppResponseMessage::BootNotification(response),
                        Err(error) => error.into_ocpp_response(),
                    }
                } else {
                    match DefaultBootNotificationHandler.handle(request).await {
                        Ok(response) => OcppResponseMessage::BootNotification(response),
                        Err(error) => error.into_ocpp_response(),
                    }
                }
            }
            OcppRequestMessage::Heartbeat(request) => {
                if let Some(handler) = &self.heartbeat_handler {
                    match handler.handle(request).await {
                        Ok(response) => OcppResponseMessage::Heartbeat(response),
                        Err(error) => error.into_ocpp_response(),
                    }
                } else {
                    match DefaultHeartbeatHandler.handle(request).await {
                        Ok(response) => OcppResponseMessage::Heartbeat(response),
                        Err(error) => error.into_ocpp_response(),
                    }
                }
            }
        }
    }
}

async fn run_controller(mut controller_actor: Controller) -> CrushResult<()> {
    while let Some(msg) = controller_actor.receiver.recv().await {
        if let Err(error) = controller_actor.handle_message(msg).await {
            tracing::error!("{error}");
        };
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
        status_notification_handler: Option<Box<dyn HandleStatusNotificationRequest + Send + Sync>>,
    ) -> Self {
        let (sender, receiver) = channel(64);

        tokio::spawn(async move {
            let actor = Controller::new(
                receiver,
                heartbeat_handler,
                boot_notification_handler,
                status_notification_handler,
            );
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
