import { invoke } from "@tauri-apps/api/core";

// 백엔드 config.rs AppConfig 와 1:1. 자격증명 없음(포트/슬롯수/주입지연/컨텍스트 압축).
export type AppConfig = {
  rest_port: number;
  slots: number;
  inject_delay_ms: number;
  /** 스텝 전이 시 컨텍스트 점유율(%)이 이 값 이상이면 /compact 주입. 기본 40. */
  compact_threshold_pct: number;
  /** 컨텍스트 윈도우 토큰(점유율 분모). 기본 1,000,000. */
  context_window_tokens: number;
};

export const loadConfig = () => invoke<AppConfig>("get_config");
