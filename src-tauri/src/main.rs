#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod websocket;
mod keychain;
mod ollama;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, CustomMenuItem};

static APP_STATE: Lazy<Arc<RwLock<AppState>>> = Lazy::new(|| {
    Arc::new(RwLock::new(AppState::default()))
});

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub client_id: Option<String>,
    pub operator_id: Option<String>,
    pub server_url: String,
    pub ollama_url: String,
    pub connected: bool,
    pub last_heartbeat: Option<String>,
    pub models: Vec<String>,
    pub current_load: u32,
    pub total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub client_id: String,
    pub api_secret: String,
    pub server_url: String,
    pub ollama_url: String,
}

#[tauri::command]
async fn get_status() -> Result<AppState, String> {
    Ok(APP_STATE.read().clone())
}

#[tauri::command]
async fn save_credentials(config: ConnectionConfig) -> Result<String, String> {
    keychain::store_credentials(&config.client_id, &config.api_secret)
        .map_err(|e| format!("Failed to store credentials: {}", e))?;
    
    let mut state = APP_STATE.write();
    state.client_id = Some(config.client_id);
    state.server_url = config.server_url;
    state.ollama_url = config.ollama_url;
    
    Ok("Credentials saved securely".to_string())
}

#[tauri::command]
async fn load_credentials() -> Result<Option<(String, String)>, String> {
    let state = APP_STATE.read();
    if let Some(client_id) = &state.client_id {
        match keychain::get_credentials(client_id) {
            Ok(secret) => Ok(Some((client_id.clone(), secret))),
            Err(_) => Ok(None),
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn connect(app: tauri::AppHandle) -> Result<String, String> {
    let state = APP_STATE.read().clone();
    
    let client_id = state.client_id.clone()
        .ok_or("No client ID configured")?;
    let api_secret = keychain::get_credentials(&client_id)
        .map_err(|e| format!("Failed to get credentials: {}", e))?;
    
    let server_url = if state.server_url.is_empty() {
        "wss://aiassist-secure.replit.app/api/v1/pin/ws".to_string()
    } else {
        state.server_url.replace("https://", "wss://").replace("http://", "ws://")
            + "/api/v1/pin/ws"
    };
    
    let ollama_url = if state.ollama_url.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        state.ollama_url.clone()
    };
    
    tokio::spawn(async move {
        if let Err(e) = websocket::connect_to_server(&server_url, &client_id, &api_secret, &ollama_url).await {
            log::error!("WebSocket connection error: {}", e);
            let mut state = APP_STATE.write();
            state.connected = false;
        }
    });
    
    Ok("Connecting...".to_string())
}

#[tauri::command]
async fn disconnect() -> Result<String, String> {
    websocket::disconnect().await;
    let mut state = APP_STATE.write();
    state.connected = false;
    Ok("Disconnected".to_string())
}

#[tauri::command]
async fn test_ollama(url: String) -> Result<Vec<String>, String> {
    ollama::test_connection(&url).await
}

#[tauri::command]
async fn get_ollama_models() -> Result<Vec<String>, String> {
    let state = APP_STATE.read();
    let url = if state.ollama_url.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        state.ollama_url.clone()
    };
    ollama::get_models(&url).await
}

fn create_tray_menu() -> SystemTrayMenu {
    SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("status", "Status: Disconnected").disabled())
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("connect", "Connect"))
        .add_item(CustomMenuItem::new("disconnect", "Disconnect").disabled())
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("show", "Show Window"))
        .add_item(CustomMenuItem::new("quit", "Quit"))
}

fn main() {
    env_logger::init();
    
    let tray = SystemTray::new().with_menu(create_tray_menu());
    
    tauri::Builder::default()
        .system_tray(tray)
        .on_system_tray_event(|app, event| {
            match event {
                SystemTrayEvent::LeftClick { .. } => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                SystemTrayEvent::MenuItemClick { id, .. } => {
                    match id.as_str() {
                        "show" => {
                            if let Some(window) = app.get_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "connect" => {
                            let app_handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = connect(app_handle).await;
                            });
                        }
                        "disconnect" => {
                            tauri::async_runtime::spawn(async {
                                let _ = disconnect().await;
                            });
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            save_credentials,
            load_credentials,
            connect,
            disconnect,
            test_ollama,
            get_ollama_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
