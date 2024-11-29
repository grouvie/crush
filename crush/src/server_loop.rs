use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use crate::{
    client_loop::{ClientHandle, ToClient},
    controller_loop::{ControllerHandle, ToController},
    error::CrushResult,
};

pub(crate) enum ToServer {
    NewClient(ClientHandle),
    ClientGone(usize),
    ClientMessage(usize, String),
}

struct Server {
    controller_handle: ControllerHandle,
    receiver: Receiver<ToServer>,
    clients: HashMap<usize, ClientHandle>,
}

impl Server {
    fn new(controller_handle: ControllerHandle, receiver: Receiver<ToServer>) -> Self {
        Self {
            controller_handle,
            receiver,
            clients: HashMap::default(),
        }
    }
    async fn handle_message(&mut self, msg: ToServer) -> CrushResult<()> {
        match msg {
            ToServer::NewClient(client_handle) => {
                self.clients.insert(client_handle.id, client_handle);
            }
            ToServer::ClientGone(id) => {
                tracing::info!("Client with {id} disconnected");
                drop(self.clients.remove(&id));
            }
            ToServer::ClientMessage(id, message) => {
                tracing::info!("{message}");

                if let Some(client_handle) = self.clients.get_mut(&id) {
                    let (sender, receiver) = oneshot::channel();
                    let to_controller = ToController::Message(message, sender);
                    self.controller_handle.send(to_controller).await;

                    let response = receiver.await?;

                    let to_client = ToClient::Message(response);

                    client_handle.send(to_client).await;
                }
            }
        };
        Ok(())
    }
}

async fn run_server(mut server_actor: Server) -> CrushResult<()> {
    while let Some(msg) = server_actor.receiver.recv().await {
        server_actor.handle_message(msg).await?;
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) struct ServerHandle {
    sender: Sender<ToServer>,
    next_id: Arc<AtomicUsize>,
}

impl ServerHandle {
    pub(crate) fn new(controller_handle: ControllerHandle) -> (Self, JoinHandle<()>) {
        let (sender, receiver) = channel(64);

        let actor = Server::new(controller_handle, receiver);

        let server_join = tokio::spawn(async move {
            if let Err(error) = run_server(actor).await {
                tracing::error!("{error}");
            }
        });

        (
            Self {
                sender,
                next_id: Arc::default(),
            },
            server_join,
        )
    }
    pub(crate) fn next_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }
    pub(crate) async fn send(&mut self, msg: ToServer) {
        assert!(
            self.sender.send(msg).await.is_ok(),
            "Server loop has shut down"
        );
    }
}
