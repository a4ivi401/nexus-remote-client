use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "register")]
    Register,

    #[serde(rename = "connect_request")]
    ConnectRequest { target_id: String },

    #[serde(rename = "connect_response")]
    ConnectResponse {
        requester_id: String,
        accepted: bool,
    },

    #[serde(rename = "signal")]
    Signal { target_id: String, data: String },

    #[serde(rename = "disconnect")]
    Disconnect { target_id: String },

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "ping")]
    Ping,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TurnCredentials {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "registered")]
    Registered {
        session_id: String,
        turn: TurnCredentials,
    },

    #[serde(rename = "connection_request")]
    ConnectionRequest { requester_id: String },

    #[serde(rename = "connection_accepted")]
    ConnectionAccepted { peer_id: String },

    #[serde(rename = "connection_rejected")]
    ConnectionRejected { peer_id: String, reason: String },

    #[serde(rename = "signal")]
    Signal { from_id: String, data: String },

    #[serde(rename = "peer_disconnected")]
    PeerDisconnected { peer_id: String },

    #[serde(rename = "error")]
    Error { code: String, message: String },

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "pong")]
    Pong,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum WebRtcSignalData {
    #[serde(rename = "offer")]
    Offer { sdp: String },
    #[serde(rename = "answer")]
    Answer { sdp: String },
    #[serde(rename = "candidate")]
    Candidate {
        candidate: String,
        sdp_mline_index: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        sdp_mid: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let reg = ClientMessage::Register;
        assert_eq!(serde_json::to_string(&reg).unwrap(), r#"{"type":"register"}"#);

        let req = ClientMessage::ConnectRequest {
            target_id: "123456".to_string(),
        };
        assert_eq!(
            serde_json::to_string(&req).unwrap(),
            r#"{"type":"connect_request","target_id":"123456"}"#
        );

        let resp = ClientMessage::ConnectResponse {
            requester_id: "654321".to_string(),
            accepted: true,
        };
        assert_eq!(
            serde_json::to_string(&resp).unwrap(),
            r#"{"type":"connect_response","requester_id":"654321","accepted":true}"#
        );
    }

    #[test]
    fn test_server_message_deserialization() {
        let json_registered = r#"{
            "type": "registered",
            "session_id": "849201",
            "turn": {
                "urls": ["stun:stun.l.google.com:19302", "turn:turn.example.com:3478"],
                "username": "user1",
                "credential": "pass1"
            }
        }"#;

        let msg: ServerMessage = serde_json::from_str(json_registered).unwrap();
        match msg {
            ServerMessage::Registered { session_id, turn } => {
                assert_eq!(session_id, "849201");
                assert_eq!(turn.urls.len(), 2);
                assert_eq!(turn.username, "user1");
            }
            _ => panic!("Expected Registered message"),
        }

        let json_conn_req = r#"{"type":"connection_request","requester_id":"102938"}"#;
        let msg: ServerMessage = serde_json::from_str(json_conn_req).unwrap();
        assert_eq!(
            msg,
            ServerMessage::ConnectionRequest {
                requester_id: "102938".to_string()
            }
        );
    }

    #[test]
    fn test_webrtc_signal_data() {
        let offer = WebRtcSignalData::Offer {
            sdp: "v=0...".to_string(),
        };
        let serialized = serde_json::to_string(&offer).unwrap();
        assert_eq!(serialized, r#"{"type":"offer","sdp":"v=0..."}"#);

        let cand = WebRtcSignalData::Candidate {
            candidate: "candidate:1...".to_string(),
            sdp_mline_index: 0,
            sdp_mid: None,
        };
        let cand_str = serde_json::to_string(&cand).unwrap();
        assert_eq!(
            cand_str,
            r#"{"type":"candidate","candidate":"candidate:1...","sdp_mline_index":0}"#
        );
    }
}
