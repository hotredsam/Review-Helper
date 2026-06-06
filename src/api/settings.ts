import { invoke } from "@tauri-apps/api/core";

export type ProviderKind = "claude" | "local";

/** Mirrors the Rust `ModelConfig`. Persisted in the SQLite settings table. */
export interface ModelConfig {
  provider: ProviderKind;
  local_endpoint: string | null;
  api_credit_overflow: boolean;
}

export function getModelConfig(): Promise<ModelConfig> {
  return invoke<ModelConfig>("get_model_config");
}

export function setModelConfig(config: ModelConfig): Promise<void> {
  return invoke("set_model_config", { config });
}
