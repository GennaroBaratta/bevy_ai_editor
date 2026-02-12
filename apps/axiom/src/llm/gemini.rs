use serde::{Deserialize, Serialize};
use anyhow::{Context as _, Result};
use reqwest::Client;
use serde_json::Value;
use futures_util::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct GeminiClient {
    api_key: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentPart {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: Option<MessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Choice {
    pub message: Message,
}

// Stream types
#[derive(Debug, Clone)]
pub enum StreamEvent {
    TextChunk(String),
    ToolCallChunk(StreamDeltaToolCall),
    Done,
}

#[derive(Deserialize, Debug)]
pub struct StreamChunk {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub choices: Vec<StreamChoice>,
}

#[derive(Deserialize, Debug)]
pub struct StreamChoice {
    pub delta: StreamDelta,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct StreamDelta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<StreamDeltaToolCall>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StreamDeltaToolCall {
    #[allow(dead_code)]
    pub index: i32,
    pub id: Option<String>,
    pub r#type: Option<String>,
    pub function: Option<StreamDeltaFunction>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StreamDeltaFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

impl GeminiClient {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        let builder = Client::builder();
        
        println!("GeminiClient::new called");
        
        // We only configure proxy if explicitly set, otherwise we trust the local rotation proxy
        // which the user provided (http://127.0.0.1:8045).
        // Since that local proxy is an OpenAI adapter, we likely don't need an upstream HTTPS_PROXY for it
        // unless it's running on a different machine (unlikely for 127.0.0.1).
        
        if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
             // Only apply if the target isn't localhost/127.0.0.1, OR if the user really wants it.
             // But usually for local dev we don't proxy localhost.
             // Assuming the user might have set it for other things.
             // Let's just log it for now.
             println!("HTTPS_PROXY env var found: {}", proxy_url);
        }

        let client = builder
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self {
            api_key,
            model,
            client,
        })
    }

    pub async fn chat_completion_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Value>>,
    ) -> Result<impl Stream<Item = Result<StreamEvent>>> {
        // Default to local proxy as requested by user example
        let base_url = std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8045/v1".to_string());
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        println!("Sending OpenAI-compatible STREAM request to: {}", url);

        let request_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools,
            stream: Some(true),
        };

        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;
        let mut base_delay = 2; // seconds

        loop {
            let response = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&request_body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        println!("Successfully sent stream request");
                        let stream = resp.bytes_stream();
                        return Ok(SseStream::new(stream));
                    } else if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS || resp.status().as_u16() == 429 {
                        if retry_count >= MAX_RETRIES {
                            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            return Err(anyhow::anyhow!("API error (Rate Limit Exceeded): {}", error_text));
                        }
                        println!("Rate limited (429). Retrying in {} seconds...", base_delay);
                        sleep(Duration::from_secs(base_delay)).await;
                        retry_count += 1;
                        base_delay *= 2; // Exponential backoff
                        continue;
                    } else {
                        // Check for other errors (like 500) that might be transient
                         if resp.status().is_server_error() && retry_count < MAX_RETRIES {
                            println!("Server error ({}). Retrying in {} seconds...", resp.status(), base_delay);
                            sleep(Duration::from_secs(base_delay)).await;
                            retry_count += 1;
                            base_delay *= 2; 
                            continue;
                         }
                        
                        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        return Err(anyhow::anyhow!("API error: {}", error_text));
                    }
                }
                Err(e) => {
                    println!("Failed to send stream request: {}", e);
                    if retry_count >= MAX_RETRIES {
                         return Err(anyhow::anyhow!("Network error: {}", e));
                    }
                    sleep(Duration::from_secs(base_delay)).await;
                    retry_count += 1;
                    base_delay *= 2;
                }
            }
        }
    }
}

#[allow(dead_code)]
impl GeminiClient {
    pub async fn chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Value>>,
    ) -> Result<ChatCompletionResponse> {
        // Default to local proxy as requested by user example
        let base_url = std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8045/v1".to_string());
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        println!("Sending OpenAI-compatible request to: {}", url);

        let request_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            tools,
            stream: None,
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await;

        match &response {
            Ok(_) => println!("Successfully sent request"),
            Err(e) => println!("Failed to send request: {}", e),
        }

        let response = response.context("Failed to send request to API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        let response_body: ChatCompletionResponse = response.json().await
            .context("Failed to parse API response")?;

        Ok(response_body)
    }
}


pub struct SseStream<S> {
    inner: S,
    buffer: Vec<u8>,
}

impl<S> SseStream<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
        }
    }
}

impl<S, B> Stream for SseStream<S>
where
    S: Stream<Item = reqwest::Result<B>> + Unpin,
    B: AsRef<[u8]>,
{
    type Item = Result<StreamEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // Check buffer for newline
            if let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
                let line_bytes = self.buffer.drain(..pos + 1).collect::<Vec<u8>>();
                let line_str = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]).to_string();
                let line = line_str.trim();

                if !line.is_empty() {
                     // println!("SSE Received Line: {}", line);
                }

                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        return Poll::Ready(Some(Ok(StreamEvent::Done)));
                    }

                    // Parse OpenAI-compatible Stream Response
                    match serde_json::from_str::<StreamChunk>(data) {
                        Ok(chunk) => {
                             if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    if !content.is_empty() {
                                        return Poll::Ready(Some(Ok(StreamEvent::TextChunk(content.clone()))));
                                    }
                                }
                                if let Some(tool_calls) = &choice.delta.tool_calls {
                                    if let Some(tool_call) = tool_calls.first() {
                                        return Poll::Ready(Some(Ok(StreamEvent::ToolCallChunk(tool_call.clone()))));
                                    }
                                }
                            }
                        }
                        Err(_e) => {
                             // Ignore parse errors
                        }
                    }
                }
                continue;
            }

            // No newline found, pull more data
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buffer.extend_from_slice(chunk.as_ref());
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(anyhow::Error::from(e))));
                },
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                },
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
