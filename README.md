# PIN Client

P2P Inference Network Operator Client - A Tauri-based desktop application that connects your Ollama instance to the PIN network.

## Overview

The PIN Client allows GPU/NPU operators to offer inference services through the P2P Inference Network. It:

1. **Securely stores credentials** in your OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
2. **Maintains a persistent WebSocket connection** to the PIN server
3. **Proxies inference requests** to your local Ollama instance
4. **Reports health metrics** (load, capacity, models) to the network

## Prerequisites

- [Ollama](https://ollama.ai) installed and running locally
- At least one model pulled (e.g., `ollama pull llama3.2`)
- PIN operator credentials (client_id and api_secret from registration)

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/aiassistsecure/pin-client/releases) page.

### Build from Source

1. Install Rust and Tauri prerequisites:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # macOS
   xcode-select --install
   
   # Ubuntu/Debian
   sudo apt update
   sudo apt install libwebkit2gtk-4.0-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
   
   # Windows
   # Install Visual Studio Build Tools
   ```

2. Clone and build:
   ```bash
   cd pin-client
   cargo tauri build
   ```

## Configuration

1. Launch the PIN Client
2. Enter your credentials:
   - **Client ID**: Your `pin_xxxx` identifier from registration
   - **API Secret**: Your secret key (stored securely in OS keychain)
   - **Ollama URL**: Usually `http://localhost:11434`
   - **Server URL**: Leave blank for production, or enter custom server

3. Click "Test Ollama" to verify your Ollama connection
4. Click "Save & Connect" to join the network

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         PIN Client                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │  UI (HTML)  │  │  Keychain   │  │    WebSocket Handler    │ │
│  │             │  │  Storage    │  │                         │ │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘ │
│         │                │                     │               │
│         └────────────────┼─────────────────────┤               │
│                          │                     │               │
│  ┌───────────────────────┴─────────────────────┴─────────────┐ │
│  │                    Tauri Commands                          │ │
│  └───────────────────────────────────────────────────────────┘ │
│                          │                                     │
│  ┌───────────────────────┴─────────────────────────────────┐   │
│  │                  Ollama Proxy                            │   │
│  └───────────────────────────────────────────────────────────┘  │
└────────────────────────────────┬───────────────────────────────┘
                                 │
                    ┌────────────┴────────────┐
                    │    Local Ollama         │
                    │    (localhost:11434)    │
                    └─────────────────────────┘
```

## Message Protocol

### Authentication
```json
{
  "type": "AUTH",
  "client_id": "pin_xxxx",
  "timestamp": "1704499200",
  "signature": "sha256(client_id + timestamp + secret_hash)"
}
```

### Heartbeat
```json
{ "type": "HEARTBEAT" }
// Response: { "type": "HEARTBEAT_ACK" }
```

### Model List Update
```json
{
  "type": "MODEL_LIST",
  "models": ["llama3.2", "mistral", "codellama"]
}
```

### Inference Request (from server)
```json
{
  "type": "INFERENCE_REQUEST",
  "request_id": "req_abc123",
  "payload": {
    "model": "llama3.2",
    "messages": [{"role": "user", "content": "Hello"}]
  }
}
```

### Inference Response
```json
{
  "type": "INFERENCE_RESPONSE",
  "request_id": "req_abc123",
  "result": {
    "model": "llama3.2",
    "message": {"role": "assistant", "content": "Hi!"},
    "done": true
  }
}
```

## System Tray

The PIN Client runs in your system tray with the following options:
- **Status**: Shows current connection state
- **Connect/Disconnect**: Toggle network connection
- **Show Window**: Open the main window
- **Quit**: Exit the application

## Security

- API secrets are stored in your OS's secure credential storage
- WebSocket connections use TLS encryption
- Authentication uses HMAC-SHA256 signatures with timestamp replay protection

## Troubleshooting

### "Failed to connect to Ollama"
- Ensure Ollama is running: `ollama serve`
- Check the URL is correct (default: `http://localhost:11434`)

### "Invalid credentials"
- Verify your client_id and api_secret from the operator dashboard
- Re-register if credentials were regenerated

### "Connection timeout"
- Check your internet connection
- Verify the server URL is accessible

