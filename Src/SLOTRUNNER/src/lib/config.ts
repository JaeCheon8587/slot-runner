import { invoke } from "@tauri-apps/api/core";

// 백엔드 config.rs AppConfig 와 1:1. 자격증명 없음(포트/슬롯수/주입지연).
export type AppConfig = {
  rest_port: number;
  slots: number;
  inject_delay_ms: number;
};

export const loadConfig = () => invoke<AppConfig>("get_config");
