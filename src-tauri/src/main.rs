#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod websocket;
mod keychain;
mod ollama;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{
    Manager,
    tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent},
    menu::{Menu, MenuItem},
};

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
async fn connect(_app: tauri::AppHandle) -> Result<String, String> {
    let (client_id, server_url, ollama_url) = {
        let state = APP_STATE.read();
        (
            state.client_id.clone(),
            state.server_url.clone(),
            state.ollama_url.clone(),
        )
    };
    
    let client_id = client_id.ok_or("No client ID configured")?;
    let api_secret = keychain::get_credentials(&client_id)
        .map_err(|e| format!("Failed to get credentials: {}", e))?;
    
    let server_url = if server_url.is_empty() {
        "wss://aiassist-secure.replit.app/api/v1/pin/ws".to_string()
    } else {
        server_url.replace("https://", "wss://").replace("http://", "ws://")
            + "/api/v1/pin/ws"
    };
    
    let ollama_url = if ollama_url.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        ollama_url
    };
    
    tauri::async_runtime::spawn(async move {
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
    websocket::disconnect();
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
    let url = {
        let state = APP_STATE.read();
        if state.ollama_url.is_empty() {
            "http://localhost:11434".to_string()
        } else {
            state.ollama_url.clone()
        }
    };
    ollama::get_models(&url).await
}

fn main() {
    env_logger::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let connect_item = MenuItem::with_id(app, "connect", "Connect", true, None::<&str>)?;
            let disconnect_item = MenuItem::with_id(app, "disconnect", "Disconnect", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[&show_item, &connect_item, &disconnect_item, &quit_item])?;
            
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
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
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            
            Ok(())
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
