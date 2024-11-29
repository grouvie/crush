use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
    Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::{
    client_loop::{ClientHandle, ClientInfo},
    error::CrushResult,
    server_loop::ServerHandle,
};

struct Accept {
    bind: SocketAddr,
    server_handle: ServerHandle,
}

impl Accept {
    fn new(bind: SocketAddr, server_handle: ServerHandle) -> Self {
        Self {
            bind,
            server_handle,
        }
    }
    async fn accept_loop(&self) -> CrushResult<()> {
        let listener = TcpListener::bind(self.bind).await?;

        loop {
            let (tcp, ip) = listener.accept().await?;
            let server_handle = self.server_handle.clone();
            tokio::spawn(async move {
                let tokio_io = TokioIo::new(tcp);

                let service =
                    service_fn(move |request| handle_request(request, ip, server_handle.clone()));

                let connection = http1::Builder::new()
                    .serve_connection(tokio_io, service)
                    .with_upgrades();

                if let Err(error) = connection.await {
                    tracing::error!("{error}");
                }
            });
        }
    }
}

async fn run_accept(accept_actor: Accept) -> CrushResult<()> {
    accept_actor.accept_loop().await?;
    Ok(())
}

pub(crate) struct AcceptHandle;

impl AcceptHandle {
    pub(crate) fn start(bind: SocketAddr, server_handle: ServerHandle) {
        let actor = Accept::new(bind, server_handle);
        tokio::spawn(async move {
            if let Err(error) = run_accept(actor).await {
                tracing::error!("{error}");
            }
        });
    }
}

#[allow(clippy::unused_async, reason = "Used for closure")]
async fn handle_request(
    mut request: Request<Incoming>,
    ip: SocketAddr,
    server_handle: ServerHandle,
) -> CrushResult<Response<Full<Bytes>>> {
    if !hyper_tungstenite::is_upgrade_request(&request) {
        let body = Full::<Bytes>::from("This endpoint requires a WebSocket upgrade request.");
        let response = Response::builder()
            .status(StatusCode::UPGRADE_REQUIRED)
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Content-Type", "text/plain")
            .body(body)?;
        return Ok(response);
    }

    let name = match extract_name(&request)? {
        ExtractNameResult::Name(name) => name,
        ExtractNameResult::Error(response) => return Ok(response),
    };

    let Ok((response, websocket)) = hyper_tungstenite::upgrade(&mut request, None) else {
        let body = Full::<Bytes>::from("WebSocket upgrade failed. Please try again.");
        let response = Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(body)?;
        return Ok(response);
    };

    tokio::spawn(async move {
        let id = server_handle.next_id();

        let client_info = ClientInfo {
            ip,
            id,
            name,
            server_handle,
            websocket,
        };

        ClientHandle::spawn(client_info);
    });

    Ok(response)
}

enum ExtractNameResult {
    Name(String),
    Error(Response<Full<Bytes>>),
}

fn extract_name(request: &Request<Incoming>) -> CrushResult<ExtractNameResult> {
    let path = request.uri().path();
    let path_segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<&str>>();

    match path_segments.as_slice() {
        ["ocpp", name] if !name.is_empty() => Ok(ExtractNameResult::Name((*name).to_owned())),
        _ => {
            let body =
                Full::<Bytes>::from("Invalid URI format. Expected: /ocpp/{charging_station_name}");

            let result = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "text/plain")
                .body(body)?;

            Ok(ExtractNameResult::Error(result))
        }
    }
}
