use crate::domain::models::{RedEyeContent, RedEyeConversation, RedEyeMessage, RedEyeRole};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Translation error: {0}")]
    TranslationError(String),
}

// ----------------------------------------------------------------------------
// OpenAI Structs
// ----------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct OpenAIChatRequest {
    pub messages: Vec<OpenAIMessage>,
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: Option<serde_json::Value>, 
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIToolCall {
    pub id: String,
    pub r#type: String, // typically "function"
    pub function: OpenAIFunctionCall,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String, // usually a JSON string
}

// ----------------------------------------------------------------------------
// Anthropic Structs
// ----------------------------------------------------------------------------
#[derive(Debug, Serialize, PartialEq)]
pub struct AnthropicRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AnthropicMessage {
    pub role: String, // "user" or "assistant"
    pub content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContent {
    Text { text: String },
    Image { source: AnthropicImageSource },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AnthropicImageSource {
    pub r#type: String,
    pub media_type: String,
    pub data: String,
}

// ----------------------------------------------------------------------------
// Translators
// ----------------------------------------------------------------------------
impl TryFrom<OpenAIChatRequest> for RedEyeConversation {
    type Error = AppError;

    fn try_from(req: OpenAIChatRequest) -> Result<Self, Self::Error> {
        let mut system_prompt = None;
        let mut redeye_messages = Vec::with_capacity(req.messages.len());

        for msg in req.messages {
            let role = match msg.role.as_str() {
                "system" => {
                    if let Some(content) = msg.content {
                        if let Some(s) = content.as_str() {
                            system_prompt = Some(s.to_string());
                        } else if let Some(arr) = content.as_array() {
                            // Extract text from part array trivially if necessary
                             for part in arr {
                                 if part.get("type").and_then(|v| v.as_str()) == Some("text") {
                                      if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                           system_prompt = Some(text.to_string());
                                           break;
                                      }
                                 }
                             }
                        }
                    }
                    continue;
                }
                "user" => RedEyeRole::User,
                "assistant" => RedEyeRole::Assistant,
                "tool" | "function" => RedEyeRole::Tool,
                other => return Err(AppError::TranslationError(format!("Unknown OpenAI role: {}", other))),
            };

            let mut contents = Vec::new();

            if let Some(content) = msg.content {
                if let Some(s) = content.as_str() {
                    if !s.is_empty() {
                         contents.push(RedEyeContent::Text { text: s.to_string() });
                    }
                } else if let Some(arr) = content.as_array() {
                    for part in arr {
                        if let Some(type_str) = part.get("type").and_then(|v| v.as_str()) {
                            if type_str == "text" {
                                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                    contents.push(RedEyeContent::Text { text: text.to_string() });
                                }
                            } else if type_str == "image_url" {
                                if let Some(url_obj) = part.get("image_url") {
                                    if let Some(url_str) = url_obj.get("url").and_then(|v| v.as_str()) {
                                        contents.push(RedEyeContent::ImageUrl { url: url_str.to_string() });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // handle tool calls from OpenAI
            if let Some(tool_calls) = msg.tool_calls {
                for tc in tool_calls {
                    let args = serde_json::from_str(&tc.function.arguments)
                        .map_err(|_| AppError::TranslationError("Invalid tool arguments JSON".to_string()))?;
                    contents.push(RedEyeContent::ToolCall {
                        id: tc.id,
                        name: tc.function.name,
                        arguments: args,
                    });
                }
            }
            
            // handle tool results
            if role == RedEyeRole::Tool {
                if let Some(tool_id) = msg.tool_call_id {
                    // Flatten any text content into the tool result string
                    let mut result_text = String::new();
                    for c in &contents {
                        if let RedEyeContent::Text { text } = c {
                            result_text.push_str(text);
                        }
                    }
                    contents.clear(); // Convert pure text blocks into the specialized ToolResult block
                    contents.push(RedEyeContent::ToolResult {
                        tool_id,
                        content: result_text,
                    });
                }
            }

            if !contents.is_empty() {
                redeye_messages.push(RedEyeMessage {
                    role,
                    content: contents,
                });
            }
        }

        Ok(RedEyeConversation {
            system_prompt,
            messages: redeye_messages,
            tools: req.tools, // Zero-copy move into the universal schema
        })
    }
}

impl TryInto<AnthropicRequest> for RedEyeConversation {
    type Error = AppError;

    fn try_into(self) -> Result<AnthropicRequest, Self::Error> {
        let mut anthropic_messages: Vec<AnthropicMessage> = Vec::with_capacity(self.messages.len());

        for msg in self.messages {
            let anthropic_role = match msg.role {
                RedEyeRole::System => {
                    continue; // System prompts are handled at the root.
                }
                RedEyeRole::User | RedEyeRole::Tool => "user".to_string(), // Tool responses are from the 'user' per Anthropic spec.
                RedEyeRole::Assistant => "assistant".to_string(),
            };

            // CRITICAL EDGE CASE FIX: Anthropic strictly requires user and assistant roles to alternate.
            // If the current role matches the last tracked role, we MUST inject a dummy alternate message.
            if let Some(last_msg) = anthropic_messages.last() {
                if last_msg.role == anthropic_role {
                    let dummy_role = if anthropic_role == "user" { "assistant" } else { "user" };
                    anthropic_messages.push(AnthropicMessage {
                        role: dummy_role.to_string(),
                        content: vec![AnthropicContent::Text { text: "<dummy>".to_string() }],
                    });
                }
            }
            
            let mut anthropic_content = Vec::with_capacity(msg.content.len());
            for c in msg.content {
                match c {
                    RedEyeContent::Text { text } => {
                        anthropic_content.push(AnthropicContent::Text { text });
                    }
                    RedEyeContent::ImageUrl { url } => {
                        // In a real scenario, this would determine media_type and encode data, but for now we map URLs.
                        anthropic_content.push(AnthropicContent::Image {
                            source: AnthropicImageSource {
                                r#type: "url".to_string(),
                                media_type: "image/jpeg".to_string(),
                                data: url,
                            }
                        });
                    }
                    RedEyeContent::ToolCall { id, name, arguments } => {
                        anthropic_content.push(AnthropicContent::ToolUse {
                            id,
                            name,
                            input: arguments, // Zero-copy move
                        });
                    }
                    RedEyeContent::ToolResult { tool_id, content } => {
                        anthropic_content.push(AnthropicContent::ToolResult {
                            tool_use_id: tool_id,
                            content, 
                        });
                    }
                }
            }

            anthropic_messages.push(AnthropicMessage {
                role: anthropic_role,
                content: anthropic_content,
            });
        }

        let mapped_tools = crate::infrastructure::schema_mapper::map_openai_tools_to_anthropic(self.tools);

        Ok(AnthropicRequest {
            system: self.system_prompt,
            messages: anthropic_messages,
            tools: mapped_tools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_openai_to_universal_translation() {
        let openai_json = json!({
            "messages": [
                {
                    "role": "system",
                    "content": "You are a test bot."
                },
                {
                    "role": "user",
                    "content": "Perform a task."
                }
            ]
        });

        let req: OpenAIChatRequest = serde_json::from_value(openai_json).unwrap();
        let conv: RedEyeConversation = req.try_into().unwrap_or_else(|e| panic!("Translation failed: {:?}", e));

        assert_eq!(conv.system_prompt.as_deref(), Some("You are a test bot."));
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].role, RedEyeRole::User);
        if let RedEyeContent::Text { text } = &conv.messages[0].content[0] {
            assert_eq!(text, "Perform a task.");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_universal_to_anthropic_alternation() {
        // Create [User, User, User]
        let red_eye_conv = RedEyeConversation {
            system_prompt: None,
            tools: None,
            messages: vec![
                RedEyeMessage {
                    role: RedEyeRole::User,
                    content: vec![RedEyeContent::Text { text: "msg1".to_string() }]
                },
                RedEyeMessage {
                    role: RedEyeRole::User,
                    content: vec![RedEyeContent::Text { text: "msg2".to_string() }]
                },
                RedEyeMessage {
                    role: RedEyeRole::User,
                    content: vec![RedEyeContent::Text { text: "msg3".to_string() }]
                },
            ]
        };

        let anthropic_req: AnthropicRequest = red_eye_conv.try_into().unwrap();
        
        let roles: Vec<String> = anthropic_req.messages.iter().map(|m| m.role.clone()).collect();
        assert_eq!(roles, vec!["user", "assistant", "user", "assistant", "user"]);
        
        // Ensure dummies exist
        if let AnthropicContent::Text { text } = &anthropic_req.messages[1].content[0] {
            assert_eq!(text, "<dummy>");
        } else {
            panic!("Expected dummy text");
        }
    }

    #[test]
    fn test_tool_call_translation() {
        // Full round-trip mock testing tools
        let openai_req = OpenAIChatRequest {
            tools: Some(vec![json!({"type": "function", "function": {"name": "get_weather"}})]),
            messages: vec![
                OpenAIMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_call_id: None,
                    tool_calls: Some(vec![
                        OpenAIToolCall {
                            id: "call_123".to_string(),
                            r#type: "function".to_string(),
                            function: OpenAIFunctionCall {
                                name: "get_weather".to_string(),
                                arguments: "{\"loc\": \"Paris\"}".to_string(),
                            }
                        }
                    ]),
                }
            ]
        };

        // -> to universal schema
        let conv: RedEyeConversation = openai_req.try_into().expect("Failed TryFrom");
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].role, RedEyeRole::Assistant);
        
        let is_tool_call_correct = matches!(
            &conv.messages[0].content[0],
            RedEyeContent::ToolCall { id, name, arguments } 
            if id == "call_123" && name == "get_weather" && arguments.get("loc").and_then(|v| v.as_str()) == Some("Paris")
        );
        assert!(is_tool_call_correct, "Tool call translation into universal failed");

        // -> to anthropic request
        let anthropic: AnthropicRequest = conv.try_into().expect("Failed TryInto");
        assert_eq!(anthropic.messages.len(), 1);
        assert!(anthropic.tools.is_some());
        
        let is_anthropic_tool = matches!(
            &anthropic.messages[0].content[0],
            AnthropicContent::ToolUse { id, name, input }
            if id == "call_123" && name == "get_weather" && input.get("loc").and_then(|v| v.as_str()) == Some("Paris")
        );
        assert!(is_anthropic_tool, "Tool call translation to Anthropic failed");
    }
}
