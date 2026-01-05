use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelsResponse {
    pub models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaChatResponse {
    pub model: String,
    pub message: ChatMessage,
    pub done: bool,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u32>,
    #[serde(default)]
    pub eval_count: Option<u32>,
}

pub async fn test_connection(url: &str) -> Result<Vec<String>, String> {
    let client = Client::new();
    let api_url = format!("{}/api/tags", url.trim_end_matches('/'));
    
    let response = client
        .get(&api_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Ollama returned status: {}", response.status()));
    }
    
    let data: OllamaModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let model_names: Vec<String> = data.models.iter().map(|m| m.name.clone()).collect();
    
    log::info!("Found {} models at {}", model_names.len(), url);
    Ok(model_names)
}

pub async fn get_models(url: &str) -> Result<Vec<String>, String> {
    test_connection(url).await
}

pub async fn chat_completion(
    url: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    stream: bool,
) -> Result<OllamaChatResponse, String> {
    let client = Client::new();
    let api_url = format!("{}/api/chat", url.trim_end_matches('/'));
    
    let request = OllamaChatRequest {
        model: model.to_string(),
        messages,
        stream: Some(stream),
        options: None,
    };
    
    let response = client
        .post(&api_url)
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama error {}: {}", status, body));
    }
    
    let result: OllamaChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;
    
    Ok(result)
}

pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as f32 / 4.0).ceil() as u32
}
