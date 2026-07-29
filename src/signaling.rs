use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action")]
pub enum ServerMessage {
    #[serde(rename = "join")]
    Join { pin: String },
    #[serde(rename = "signal")]
    Signal { pin: String, data: String },
}

pub struct SignalingClient {
    pub tx: mpsc::Sender<String>,
    pub rx: mpsc::Receiver<String>,
}

impl SignalingClient {
    pub async fn connect(url: &str, pin: String) -> Result<Self, Box<dyn Error>> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send Join message
        let join_msg = ServerMessage::Join { pin: pin.clone() };
        let join_text = serde_json::to_string(&join_msg)?;
        write.send(Message::Text(join_text.into())).await?;

        let (tx_out, mut rx_out) = mpsc::channel::<String>(100);
        let (tx_in, rx_in) = mpsc::channel::<String>(100);

        // Spawn a task to read from WebSocket and send to tx_in
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    // Try to parse the signal message from the server (which is actually just raw text from the server because the server broadcasts the `data` directly to the peer).
                    // Wait, the server sends exactly what was in `data`. 
                    // So `text` is the encrypted SDP message.
                    let _ = tx_in.send(text.to_string()).await;
                }
            }
        });

        // Spawn a task to read from rx_out and send to WebSocket
        tokio::spawn(async move {
            while let Some(data) = rx_out.recv().await {
                let sig_msg = ServerMessage::Signal {
                    pin: pin.clone(),
                    data,
                };
                if let Ok(sig_text) = serde_json::to_string(&sig_msg) {
                    if write.send(Message::Text(sig_text.into())).await.is_err() {
                        break;
                    }
                }
            }
        });

        Ok(Self {
            tx: tx_out,
            rx: rx_in,
        })
    }
}
