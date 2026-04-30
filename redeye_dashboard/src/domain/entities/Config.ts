export interface LlmModel {
  id: string;
  model_name: string;
  provider_name: string;
  base_url: string;
}

export interface ProviderKey {
  id: string;
  provider_name: string;
  key_alias: string;
  api_key_masked: string;
  priority: number;
  is_active: boolean;
}
