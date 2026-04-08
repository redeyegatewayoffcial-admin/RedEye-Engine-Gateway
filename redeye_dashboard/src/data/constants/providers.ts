export interface ProviderInfo {
  id: string;
  name: string;
  format: string;
}

export const SUPPORTED_PROVIDERS: ProviderInfo[] = [
  { id: "openai", name: "OpenAI", format: "openai" },
  { id: "anthropic", name: "Anthropic", format: "anthropic" },
  { id: "gemini", name: "Google Gemini", format: "gemini" },
  { id: "groq", name: "Groq", format: "openai" },
  { id: "openrouter", name: "OpenRouter (150+ Models)", format: "openai" },
  { id: "deepseek", name: "DeepSeek", format: "openai" },
  { id: "together", name: "Together AI", format: "openai" },
  { id: "mistral", name: "Mistral AI", format: "openai" },
  { id: "xai", name: "xAI (Grok)", format: "openai" },
  { id: "cerebras", name: "Cerebras", format: "openai" },
  { id: "fireworks", name: "Fireworks AI", format: "openai" },
  { id: "siliconflow", name: "SiliconFlow", format: "openai" },
  { id: "perplexity", name: "Perplexity AI", format: "openai" },
  { id: "cohere", name: "Cohere", format: "openai" },
];
