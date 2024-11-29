use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt, TryStreamExt,
};
use hyper::upgrade::Upgraded;
use hyper_tungstenite::{tungstenite::Message, HyperWebsocket, WebSocketStream};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::try_join;
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use crate::{
    error::CrushResult,
    server_loop::{ServerHandle, ToServer},
};

pub(crate) struct ClientInfo {
    pub ip: SocketAddr,
    pub id: usize,
    pub name: String,
    pub server_handle: ServerHandle,
    pub websocket: HyperWebsocket,
}

pub(crate) enum ToClient {
    Message(String),
}

struct Client {
    id: usize,
    server_handle: ServerHandle,
    receiver: Receiver<ToClient>,
    websocket: HyperWebsocket,
}

impl Client {
    fn new(
        id: usize,
        server_handle: ServerHandle,
        receiver: Receiver<ToClient>,
        websocket: HyperWebsocket,
    ) -> Self {
        Self {
            id,
            server_handle,
            receiver,
            websocket,
        }
    }
}

pub(crate) struct ClientHandle {
    pub(crate) id: usize,
    ip: SocketAddr,
    name: String,
    sender: Sender<ToClient>,
    client_join: JoinHandle<()>,
}

impl ClientHandle {
    pub(crate) fn spawn(client_info: ClientInfo) {
        let (sender, receiver) = channel(64);

        let actor = Client::new(
            client_info.id,
            client_info.server_handle,
            receiver,
            client_info.websocket,
        );

        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        let client_join = tokio::spawn(async move {
            if let Err(error) = start_client(actor, oneshot_receiver).await {
                tracing::error!("{error}");
            };
        });

        let client_handle = ClientHandle {
            id: client_info.id,
            ip: client_info.ip,
            name: client_info.name,
            sender,
            client_join,
        };

        drop(oneshot_sender.send(client_handle));
    }
    pub(crate) async fn send(&mut self, msg: ToClient) {
        assert!(
            self.sender.send(msg).await.is_ok(),
            "Client loop has shut down"
        );
    }
}

impl Drop for ClientHandle {
    fn drop(&mut self) {
        self.client_join.abort();
    }
}

async fn start_client(
    mut client_actor: Client,
    oneshot_receiver: oneshot::Receiver<ClientHandle>,
) -> CrushResult<()> {
    let client_handle = oneshot_receiver.await?;

    tracing::info!(
        "Station: {} with IP: {} connected.",
        client_handle.name,
        client_handle.ip
    );

    client_actor
        .server_handle
        .send(ToServer::NewClient(client_handle))
        .await;

    run_client(client_actor).await?;

    Ok(())
}

async fn run_client(client_actor: Client) -> CrushResult<()> {
    let (write, read) = client_actor.websocket.await?.split();

    let ((), ()) = try_join!(
        tcp_read(client_actor.id, read, client_actor.server_handle),
        tcp_write(write, client_actor.receiver),
    )?;
    Ok(())
}

async fn tcp_read(
    id: usize,
    mut read: SplitStream<WebSocketStream<TokioIo<Upgraded>>>,
    mut server_handle: ServerHandle,
) -> CrushResult<()> {
    while let Ok(Some(message)) = read.try_next().await {
        match message {
            Message::Text(text) => server_handle.send(ToServer::ClientMessage(id, text)).await,
            Message::Close(close_frame) => {
                tracing::info!(
                    "Received close for id {} with close frame: {:?}",
                    id,
                    close_frame
                );
                server_handle.send(ToServer::ClientGone(id)).await;
            }
            Message::Frame(frame) => {
                tracing::info!("Frame: {frame}");
            }
            _ => {
                tracing::debug!("Unsupported: {message}");
            }
        }
    }
    Ok(())
}

async fn tcp_write(
    mut write: SplitSink<WebSocketStream<TokioIo<Upgraded>>, Message>,
    mut receiver: Receiver<ToClient>,
) -> CrushResult<()> {
    while let Some(msg) = receiver.recv().await {
        match msg {
            ToClient::Message(message) => write.send(message.into()).await?,
        }
    }
    Ok(())
}
