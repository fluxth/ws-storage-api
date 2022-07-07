use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use anyhow::{bail, Result};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use unwrap_or::unwrap_ok_or;
use warp::ws::{Message, WebSocket};
use warp::Filter;

// Global unique client id counter
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

// State of currently connected client
type Client = mpsc::UnboundedSender<Message>;
type ClientMap = Arc<RwLock<HashMap<usize, Client>>>;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: String,
    username: String,
    password: String,
    profile_image: String,
    // NOTE: In production this would be a `DateTime<Utc>`
    joined_date: String,
}

// Data store of all users
// NOTE: In production this would be a persistent database
type DataStore = Arc<RwLock<Vec<User>>>;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    Get,
    Add { data: User },
    Edit { id: String, data: User },
    Delete { id: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response<'a> {
    Reload { data: &'a [User] },
    Append { data: &'a User },
    Error { message: String },
}

#[tokio::main]
async fn main() {
    let clients = ClientMap::default();
    let clients = warp::any().map(move || clients.clone());

    let data_store = DataStore::default();
    let data_store = warp::any().map(move || data_store.clone());

    // websocket upgrade
    let ws_endpoint = warp::path("user")
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(clients)
        .and(data_store)
        .map(|ws: warp::ws::Ws, clients, data_store| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| client_connected(socket, clients, data_store))
        });

    let public = warp::fs::dir("./public");
    let routes = public.or(ws_endpoint);

    let port = 3030;
    eprintln!("Running server on port {}...", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}

async fn broadcast(clients: &ClientMap, payload: &Response<'_>) -> Result<()> {
    for (&_uid, client) in clients.read().await.iter() {
        let json = serde_json::to_string(payload)?;
        client.send(Message::text(json))?;
    }

    Ok(())
}

async fn send_message(client_id: usize, clients: &ClientMap, payload: &Response<'_>) -> Result<()> {
    if let Some(client) = clients.read().await.get(&client_id) {
        let json = serde_json::to_string(payload)?;
        client.send(Message::text(json))?;
    }

    Ok(())
}

async fn client_connected(ws: WebSocket, clients: ClientMap, data_store: DataStore) {
    // Use a counter to assign a new unique ID for this user
    let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("client connected: id={}", client_id);

    // Split the socket into a sender and receive of messages
    let (mut client_ws_tx, mut client_ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages to the websocket
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            client_ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln!("websocket send error: {}", e);
                })
                .await;
        }
    });

    // Save the sender in our list of connected users
    clients.write().await.insert(client_id, tx);

    while let Some(result) = client_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(client_id={}): {}", client_id, e);
                break;
            }
        };

        if let Err(error) = client_message(client_id, msg, &clients, &data_store).await {
            send_message(
                client_id,
                &clients,
                &Response::Error {
                    message: error.to_string(),
                },
            )
            .await
            .expect("send error successfully");
        }
    }

    // client_ws_rx stream will keep processing as long as the user stays connected.
    // Once they disconnect, then...
    client_disconnected(client_id, &clients).await;
}

async fn client_message(
    client_id: usize,
    msg: Message,
    clients: &ClientMap,
    data_store: &DataStore,
) -> Result<()> {
    // Skip any non-Text messages
    let msg = unwrap_ok_or!(msg.to_str(), _, bail!("Invalid request type"));

    // Parse payload
    let payload: Request = unwrap_ok_or!(
        serde_json::from_str(msg),
        _,
        bail!("Invalid request format")
    );

    match payload {
        Request::Get => {
            send_message(
                client_id,
                clients,
                &Response::Reload {
                    data: &data_store.read().await,
                },
            )
            .await?;
        }
        Request::Add { data } => {
            if data_store
                .read()
                .await
                .iter()
                .position(|item| item.id == data.id)
                .is_some()
            {
                bail!("User with ID '{}' already exists", data.id);
            }

            broadcast(clients, &Response::Append { data: &data }).await?;
            data_store.write().await.push(data);
        }
        Request::Edit { id, mut data } => {
            {
                let mut store = data_store.write().await;
                if let Some(index) = store.iter().position(|item| item.id == id) {
                    data.id = store[index].id.to_string();
                    store[index] = data;
                } else {
                    bail!("Cannot find user with ID '{}'", id);
                }
            };

            broadcast(
                clients,
                &Response::Reload {
                    data: &data_store.read().await,
                },
            )
            .await?;
        }
        Request::Delete { id } => {
            {
                let mut store = data_store.write().await;
                if let Some(index) = store.iter().position(|item| item.id == id) {
                    store.remove(index);
                } else {
                    bail!("Cannot find user with ID '{}'", id);
                }
            };

            broadcast(
                clients,
                &Response::Reload {
                    data: &data_store.read().await,
                },
            )
            .await?;
        }
    }

    Ok(())
}

async fn client_disconnected(client_id: usize, clients: &ClientMap) {
    eprintln!("client disconnected: {}", client_id);

    // Stream closed up, so remove from the user list
    clients.write().await.remove(&client_id);
}
