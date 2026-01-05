use crate::{ollama, APP_STATE};
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

static DISCONNECT_TX: Lazy<Arc<RwLock<Option<mpsc::Sender<()>>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
pub enum ServerMessage {
    AUTH_SUCCESS { operator_id: String, message: String },
    ERROR { message: String },
    PING,
    HEARTBEAT_ACK,
    MODEL_LIST_ACK,
    INFERENCE_REQUEST { request_id: String, payload: InferencePayload },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InferencePayload {
    pub model: String,
    pub messages: Vec<ollama::ChatMessage>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthMessage {
    #[serde(rename = "type")]
    msg_type: String,
    client_id: String,
    timestamp: String,
    signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<HealthReport>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthReport {
    current_load: u32,
    capacity: u32,
    gpu_memory_used: u64,
    gpu_memory_total: u64,
}

fn compute_signature(client_id: &str, timestamp: &str, api_secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_secret.as_bytes());
    let secret_hash = hex::encode(hasher.finalize());
    
    let mut sig_hasher = Sha256::new();
    sig_hasher.update(format!("{}{}{}", client_id, timestamp, secret_hash).as_bytes());
    hex::encode(sig_hasher.finalize())
}

pub async fn connect_to_server(
    server_url: &str,
    client_id: &str,
    api_secret: &str,
    ollama_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Connecting to PIN server: {}", server_url);
    
    let (ws_stream, _) = connect_async(server_url).await?;
    let (mut write, mut read) = ws_stream.split();
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    
    let signature = compute_signature(client_id, &timestamp, api_secret);
    
    let auth_msg = AuthMessage {
        msg_type: "AUTH".to_string(),
        client_id: client_id.to_string(),
        timestamp,
        signature,
    };
    
    write.send(Message::Text(serde_json::to_string(&auth_msg)?)).await?;
    log::info!("Sent AUTH message");
    
    let (disconnect_tx, mut disconnect_rx) = mpsc::channel::<()>(1);
    *DISCONNECT_TX.write() = Some(disconnect_tx);
    
    let ollama_url = ollama_url.to_string();
    
    loop {
        tokio::select! {
            _ = disconnect_rx.recv() => {
                log::info!("Disconnect signal received");
                break;
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ServerMessage>(&text) {
                            Ok(server_msg) => {
                                match server_msg {
                                    ServerMessage::AUTH_SUCCESS { operator_id, message } => {
                                        log::info!("Authenticated: {} - {}", operator_id, message);
                                        
                                        {
                                            let mut state = APP_STATE.write();
                                            state.operator_id = Some(operator_id);
                                            state.connected = true;
                                        }
                                        
                                        if let Ok(models) = ollama::get_models(&ollama_url).await {
                                            {
                                                let mut state = APP_STATE.write();
                                                state.models = models.clone();
                                            }
                                            let model_msg = ClientMessage {
                                                msg_type: "MODEL_LIST".to_string(),
                                                models: Some(models),
                                                request_id: None,
                                                result: None,
                                                error: None,
                                                health: None,
                                            };
                                            let _ = write.send(Message::Text(serde_json::to_string(&model_msg)?)).await;
                                        }
                                    }
                                    ServerMessage::ERROR { message } => {
                                        log::error!("Server error: {}", message);
                                        let mut state = APP_STATE.write();
                                        state.connected = false;
                                        break;
                                    }
                                    ServerMessage::PING => {
                                        let pong = ClientMessage {
                                            msg_type: "PONG".to_string(),
                                            request_id: None,
                                            result: None,
                                            error: None,
                                            models: None,
                                            health: None,
                                        };
                                        let _ = write.send(Message::Text(serde_json::to_string(&pong)?)).await;
                                        
                                        let mut state = APP_STATE.write();
                                        state.last_heartbeat = Some(chrono::Utc::now().to_rfc3339());
                                    }
                                    ServerMessage::HEARTBEAT_ACK | ServerMessage::MODEL_LIST_ACK => {
                                        log::debug!("Received ACK");
                                    }
                                    ServerMessage::INFERENCE_REQUEST { request_id, payload } => {
                                        log::info!("Inference request: {} for model {}", request_id, payload.model);
                                        
                                        {
                                            let mut state = APP_STATE.write();
                                            state.current_load += 1;
                                            state.total_requests += 1;
                                        }
                                        
                                        let ollama_url_clone = ollama_url.clone();
                                        let result = ollama::chat_completion(
                                            &ollama_url_clone,
                                            &payload.model,
                                            payload.messages,
                                            false,
                                        ).await;
                                        
                                        let response = match result {
                                            Ok(resp) => ClientMessage {
                                                msg_type: "INFERENCE_RESPONSE".to_string(),
                                                request_id: Some(request_id),
                                                result: Some(serde_json::to_value(resp).unwrap()),
                                                error: None,
                                                models: None,
                                                health: None,
                                            },
                                            Err(e) => ClientMessage {
                                                msg_type: "INFERENCE_ERROR".to_string(),
                                                request_id: Some(request_id),
                                                result: None,
                                                error: Some(e),
                                                models: None,
                                                health: None,
                                            },
                                        };
                                        
                                        let _ = write.send(Message::Text(serde_json::to_string(&response)?)).await;
                                        
                                        {
                                            let mut state = APP_STATE.write();
                                            state.current_load = state.current_load.saturating_sub(1);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to parse server message: {} - {}", e, text);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        log::info!("Server closed connection");
                        break;
                    }
                    Some(Err(e)) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        log::info!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    
    let mut state = APP_STATE.write();
    state.connected = false;
    *DISCONNECT_TX.write() = None;
    
    Ok(())
}

pub fn disconnect() {
    if let Some(tx) = DISCONNECT_TX.read().clone() {
        let _ = tx.blocking_send(());
    }
}
