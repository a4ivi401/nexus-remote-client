use crate::protocol::{ClientMessage, ServerMessage, TurnCredentials};
use futures::{SinkExt, StreamExt};
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

pub struct SignalingSession {
    pub session_id: String,
    pub turn: TurnCredentials,
    pub to_server_tx: mpsc::Sender<ClientMessage>,
    pub server_events_rx: mpsc::Receiver<ServerMessage>,
}

pub struct SignalingClient;

impl SignalingClient {
    pub async fn connect(url: &str) -> Result<SignalingSession, Box<dyn Error>> {
        info!("Connecting to signaling server at: {}", url);
        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            format!("Failed to connect to WebSocket server at '{}': {}", url, e)
        })?;

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // 1. Send register message
        let reg_msg = ClientMessage::Register;
        let reg_text = serde_json::to_string(&reg_msg)?;
        ws_write.send(Message::Text(reg_text.into())).await?;

        // 2. Wait for Registered response
        let mut session_id = String::new();
        let mut turn_creds = TurnCredentials {
            urls: vec![],
            username: String::new(),
            credential: String::new(),
        };

        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        if let ServerMessage::Registered { session_id: sid, turn } = server_msg {
                            info!("Successfully registered with Session ID: {}", sid);
                            session_id = sid;
                            turn_creds = turn;
                            break;
                        } else if let ServerMessage::Error { code, message } = server_msg {
                            return Err(format!("Registration error [{code}]: {message}").into());
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = ws_write.send(Message::Pong(data)).await;
                }
                Err(e) => return Err(format!("WebSocket read error during registration: {}", e).into()),
                _ => {}
            }
        }

        if session_id.is_empty() {
            return Err("Signaling server closed connection before sending registered event".into());
        }

        let (client_msg_tx, mut client_msg_rx) = mpsc::channel::<ClientMessage>(100);
        let (server_event_tx, server_event_rx) = mpsc::channel::<ServerMessage>(100);

        // Background Writer task (Client -> Server)
        let client_tx_clone = client_msg_tx.clone();
        tokio::spawn(async move {
            while let Some(cmd) = client_msg_rx.recv().await {
                if let Ok(json_str) = serde_json::to_string(&cmd) {
                    if let Err(e) = ws_write.send(Message::Text(json_str.into())).await {
                        error!("Failed to send message to signaling server: {}", e);
                        break;
                    }
                }
            }
            let _ = ws_write.close().await;
        });

        // Background Reader & Heartbeat task (Server -> Client)
        let tx_events = server_event_tx.clone();
        let tx_client = client_tx_clone.clone();
        tokio::spawn(async move {
            let mut ping_interval = tokio::time::interval(Duration::from_secs(25));
            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        if tx_client.send(ClientMessage::Ping).await.is_err() {
                            break;
                        }
                    }
                    msg = ws_read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                match serde_json::from_str::<ServerMessage>(&text) {
                                    Ok(ServerMessage::Ping) => {
                                        let _ = tx_client.send(ClientMessage::Pong).await;
                                    }
                                    Ok(ServerMessage::Pong) => {
                                        debug!("Received pong from signaling server");
                                    }
                                    Ok(server_msg) => {
                                        if tx_events.send(server_msg).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse server message '{}': {}", text, e);
                                    }
                                }
                            }
                            Some(Ok(Message::Ping(_))) => {
                                let _ = tx_client.send(ClientMessage::Pong).await;
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                info!("Signaling connection closed by server");
                                break;
                            }
                            Some(Err(e)) => {
                                error!("Signaling WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok(SignalingSession {
            session_id,
            turn: turn_creds,
            to_server_tx: client_msg_tx,
            server_events_rx: server_event_rx,
        })
    }
}
