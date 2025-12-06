use crate::error::{ProviderError, Result};
use crate::models::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Choice, ChunkChoice, Delta,
    Message, ProviderType, Tool, ToolCall, ToolChoice, Usage,
};
use serde::{Deserialize, Serialize};

/// FormatConverter handles conversion between different LLM provider formats
pub struct FormatConverter;

impl FormatConverter {
    /// Convert OpenAI format request to Anthropic format
    pub fn openai_to_anthropic(request: &ChatCompletionRequest) -> Result<AnthropicRequest> {
        let mut system_message: Option<String> = None;
        let mut messages = Vec::new();

        // Separate system messages from other messages
        for msg in &request.messages {
            if msg.role == "system" {
                system_message = msg.content.clone();
            } else {
                // Convert tool calls if present
                let content = if let Some(tool_calls) = &msg.tool_calls {
                    Self::convert_tool_calls_to_anthropic_content(tool_calls)?
                } else {
                    msg.content.clone().unwrap_or_default()
                };

                messages.push(AnthropicMessage {
                    role: msg.role.clone(),
                    content,
                });
            }
        }

        // Convert tools if present
        let tools = request
            .tools
            .as_ref()
            .map(|t| Self::convert_tools_to_anthropic(t))
            .transpose()?;

        Ok(AnthropicRequest {
            model: request.model.clone(),
            messages,
            system: system_message,
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop.clone(),
            stream: Some(request.stream),
            tools,
        })
    }

    /// Convert Anthropic format response to OpenAI format
    pub fn anthropic_to_openai(
        response: AnthropicResponse,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let mut content_parts = Vec::new();
        let mut tool_calls = Vec::new();

        // Process content blocks
        for content in response.content {
            match content {
                AnthropicContent::Text { text } => {
                    content_parts.push(text);
                }
                AnthropicContent::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: crate::models::FunctionCall {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        },
                    });
                }
            }
        }

        let content = if content_parts.is_empty() {
            None
        } else {
            Some(content_parts.join("\n"))
        };

        let message = Message {
            role: response.role,
            content,
            name: None,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
        };

        let choice = Choice {
            index: 0,
            message,
            finish_reason: Some(response.stop_reason.unwrap_or_else(|| "stop".to_string())),
            logprobs: None,
        };

        let usage = Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        Ok(ChatCompletionResponse {
            id: response.id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![choice],
            usage,
            system_fingerprint: None,
        })
    }

    /// Convert OpenAI format request to Gemini format
    pub fn openai_to_gemini(request: &ChatCompletionRequest) -> Result<GeminiRequest> {
        let mut system_instruction: Option<String> = None;
        let mut contents = Vec::new();

        for msg in &request.messages {
            if msg.role == "system" {
                system_instruction = msg.content.clone();
            } else {
                let role = match msg.role.as_str() {
                    "user" => "user",
                    "assistant" => "model",
                    _ => "user",
                };

                contents.push(GeminiContent {
                    role: role.to_string(),
                    parts: vec![GeminiPart::Text {
                        text: msg.content.clone().unwrap_or_default(),
                    }],
                });
            }
        }

        Ok(GeminiRequest {
            contents,
            system_instruction,
            generation_config: Some(GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                top_p: request.top_p,
                stop_sequences: request.stop.clone(),
            }),
        })
    }

    /// Convert Gemini format response to OpenAI format
    pub fn gemini_to_openai(
        response: GeminiResponse,
        model: &str,
    ) -> Result<ChatCompletionResponse> {
        let candidate = response
            .candidates
            .first()
            .ok_or_else(|| ProviderError::ResponseError("No candidates in response".to_string()))?;

        let content = candidate
            .content
            .parts
            .iter()
            .filter_map(|part| match part {
                GeminiPart::Text { text } => Some(text.clone()),
            })
            .collect::<Vec<_>>()
            .join("\n");

        let message = Message {
            role: "assistant".to_string(),
            content: Some(content),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let choice = Choice {
            index: 0,
            message,
            finish_reason: candidate.finish_reason.clone(),
            logprobs: None,
        };

        let usage = Usage {
            prompt_tokens: response
                .usage_metadata
                .as_ref()
                .and_then(|u| u.prompt_token_count)
                .unwrap_or(0),
            completion_tokens: response
                .usage_metadata
                .as_ref()
                .and_then(|u| u.candidates_token_count)
                .unwrap_or(0),
            total_tokens: response
                .usage_metadata
                .as_ref()
                .and_then(|u| u.total_token_count)
                .unwrap_or(0),
            prompt_tokens_details: None,
            completion_tokens_details: None,
        };

        Ok(ChatCompletionResponse {
            id: format!("gemini-{}", chrono::Utc::now().timestamp_millis()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![choice],
            usage,
            system_fingerprint: None,
        })
    }

    /// Convert streaming chunk from provider-specific format to OpenAI format
    pub fn normalize_streaming_chunk(
        chunk_data: &str,
        provider_type: ProviderType,
        model: &str,
    ) -> Result<ChatCompletionChunk> {
        match provider_type {
            ProviderType::OpenAI | ProviderType::Custom => {
                // Already in OpenAI format
                serde_json::from_str(chunk_data).map_err(|e| {
                    ProviderError::ResponseError(format!("Failed to parse OpenAI chunk: {}", e))
                        .into()
                })
            }
            ProviderType::Anthropic => Self::anthropic_chunk_to_openai(chunk_data, model),
            ProviderType::Gemini => Self::gemini_chunk_to_openai(chunk_data, model),
        }
    }

    /// Convert Anthropic streaming chunk to OpenAI format
    fn anthropic_chunk_to_openai(chunk_data: &str, model: &str) -> Result<ChatCompletionChunk> {
        let anthropic_chunk: AnthropicStreamChunk =
            serde_json::from_str(chunk_data).map_err(|e| {
                ProviderError::ResponseError(format!("Failed to parse Anthropic chunk: {}", e))
            })?;

        let delta = match anthropic_chunk.event_type.as_str() {
            "content_block_start" => Delta {
                role: Some("assistant".to_string()),
                content: None,
                tool_calls: None,
            },
            "content_block_delta" => {
                if let Some(delta_data) = anthropic_chunk.delta {
                    Delta {
                        role: None,
                        content: delta_data.text,
                        tool_calls: None,
                    }
                } else {
                    Delta {
                        role: None,
                        content: None,
                        tool_calls: None,
                    }
                }
            }
            "message_stop" => Delta {
                role: None,
                content: None,
                tool_calls: None,
            },
            _ => Delta {
                role: None,
                content: None,
                tool_calls: None,
            },
        };

        let finish_reason = if anthropic_chunk.event_type == "message_stop" {
            Some("stop".to_string())
        } else {
            None
        };

        Ok(ChatCompletionChunk {
            id: format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta,
                finish_reason,
                logprobs: None,
            }],
            system_fingerprint: None,
        })
    }

    /// Convert Gemini streaming chunk to OpenAI format
    fn gemini_chunk_to_openai(chunk_data: &str, model: &str) -> Result<ChatCompletionChunk> {
        let gemini_chunk: GeminiStreamChunk = serde_json::from_str(chunk_data).map_err(|e| {
            ProviderError::ResponseError(format!("Failed to parse Gemini chunk: {}", e))
        })?;

        let candidate = gemini_chunk.candidates.first();

        let delta = if let Some(candidate) = candidate {
            let content = candidate
                .content
                .parts
                .iter()
                .filter_map(|part| match part {
                    GeminiPart::Text { text } => Some(text.clone()),
                })
                .collect::<Vec<_>>()
                .join("");

            Delta {
                role: Some("assistant".to_string()),
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                tool_calls: None,
            }
        } else {
            Delta {
                role: None,
                content: None,
                tool_calls: None,
            }
        };

        let finish_reason = candidate.and_then(|c| c.finish_reason.clone());

        Ok(ChatCompletionChunk {
            id: format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChunkChoice {
                index: 0,
                delta,
                finish_reason,
                logprobs: None,
            }],
            system_fingerprint: None,
        })
    }

    /// Convert OpenAI tools to Anthropic format
    fn convert_tools_to_anthropic(tools: &[Tool]) -> Result<Vec<AnthropicTool>> {
        tools
            .iter()
            .map(|tool| {
                Ok(AnthropicTool {
                    name: tool.function.name.clone(),
                    description: tool.function.description.clone(),
                    input_schema: tool.function.parameters.clone(),
                })
            })
            .collect()
    }

    /// Convert tool calls to Anthropic content format
    fn convert_tool_calls_to_anthropic_content(tool_calls: &[ToolCall]) -> Result<String> {
        // For simplicity, serialize tool calls as JSON string
        // In production, you might want more sophisticated handling
        serde_json::to_string(tool_calls).map_err(|e| {
            ProviderError::ResponseError(format!("Failed to serialize tool calls: {}", e)).into()
        })
    }

    /// Convert tool choice to Anthropic format
    pub fn convert_tool_choice_to_anthropic(
        tool_choice: &ToolChoice,
    ) -> Option<AnthropicToolChoice> {
        match tool_choice {
            ToolChoice::Auto => Some(AnthropicToolChoice::Auto),
            ToolChoice::Required => Some(AnthropicToolChoice::Any),
            ToolChoice::None => None,
            ToolChoice::Specific { function, .. } => Some(AnthropicToolChoice::Tool {
                name: function.name.clone(),
            }),
        }
    }
}

// ============================================================================
// Anthropic Format Structures
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<AnthropicContent>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicStreamChunk {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<AnthropicDelta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

// ============================================================================
// Gemini Format Structures
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeminiPart {
    Text { text: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiUsageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiStreamChunk {
    pub candidates: Vec<GeminiCandidate>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, ToolFunction};

    #[test]
    fn test_openai_to_anthropic_basic() {
        let request = ChatCompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: Some("You are a helpful assistant".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message {
                    role: "user".to_string(),
                    content: Some("Hello".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            top_p: None,
            n: None,
            stream: false,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            tools: None,
            tool_choice: None,
        };

        let anthropic_req = FormatConverter::openai_to_anthropic(&request).unwrap();
        assert_eq!(anthropic_req.model, "claude-3-opus-20240229");
        assert_eq!(
            anthropic_req.system,
            Some("You are a helpful assistant".to_string())
        );
        assert_eq!(anthropic_req.messages.len(), 1);
        assert_eq!(anthropic_req.messages[0].role, "user");
        assert_eq!(anthropic_req.messages[0].content, "Hello");
    }

    #[test]
    fn test_openai_to_anthropic_with_tools() {
        let request = ChatCompletionRequest {
            model: "claude-3-opus-20240229".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Some("What's the weather?".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            n: None,
            stream: false,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            tools: Some(vec![Tool {
                r#type: "function".to_string(),
                function: ToolFunction {
                    name: "get_weather".to_string(),
                    description: Some("Get the weather".to_string()),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        }
                    }),
                },
            }]),
            tool_choice: None,
        };

        let anthropic_req = FormatConverter::openai_to_anthropic(&request).unwrap();
        assert!(anthropic_req.tools.is_some());
        let tools = anthropic_req.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "get_weather");
    }

    #[test]
    fn test_anthropic_to_openai_basic() {
        let anthropic_response = AnthropicResponse {
            id: "msg_123".to_string(),
            response_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![AnthropicContent::Text {
                text: "Hello! How can I help you?".to_string(),
            }],
            model: "claude-3-opus-20240229".to_string(),
            stop_reason: Some("end_turn".to_string()),
            usage: AnthropicUsage {
                input_tokens: 10,
                output_tokens: 20,
            },
        };

        let openai_response =
            FormatConverter::anthropic_to_openai(anthropic_response, "claude-3-opus-20240229")
                .unwrap();
        assert_eq!(openai_response.model, "claude-3-opus-20240229");
        assert_eq!(openai_response.choices.len(), 1);
        assert_eq!(
            openai_response.choices[0].message.content,
            Some("Hello! How can I help you?".to_string())
        );
        assert_eq!(openai_response.usage.prompt_tokens, 10);
        assert_eq!(openai_response.usage.completion_tokens, 20);
    }

    #[test]
    fn test_openai_to_gemini_basic() {
        let request = ChatCompletionRequest {
            model: "gemini-pro".to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: Some("You are helpful".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message {
                    role: "user".to_string(),
                    content: Some("Hello".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            top_p: None,
            n: None,
            stream: false,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            tools: None,
            tool_choice: None,
        };

        let gemini_req = FormatConverter::openai_to_gemini(&request).unwrap();
        assert_eq!(
            gemini_req.system_instruction,
            Some("You are helpful".to_string())
        );
        assert_eq!(gemini_req.contents.len(), 1);
        assert_eq!(gemini_req.contents[0].role, "user");
    }

    #[test]
    fn test_normalize_streaming_chunk_openai() {
        let chunk_json = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}"#;

        let result =
            FormatConverter::normalize_streaming_chunk(chunk_json, ProviderType::OpenAI, "gpt-4");
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.model, "gpt-4");
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }
}
