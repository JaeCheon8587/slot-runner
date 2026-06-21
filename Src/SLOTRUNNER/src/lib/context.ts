import { invoke } from "@tauri-apps/api/core";

// 슬롯 claude 세션의 컨텍스트 점유 추정 (백엔드 infra/context_usage.rs).
// 스텝 루프가 다음 스텝 전 점유율로 /compact 필요 여부를 판단한다(sidabari4loop 참고).

export type ContextUsage = {
  /** input + cache_creation + cache_read (현재 컨텍스트 토큰 합). */
  total_input_tokens: number;
  /** 마지막 응답 생성 토큰 (참고용). */
  output_tokens: number;
  /** 분모(컨텍스트 윈도우). */
  context_window: number;
};

/// 트랜스크립트에서 현재 컨텍스트 점유를 가져온다. 파일/usage 없으면 null. (Rust context_usage)
export async function getContextUsage(
  transcriptPath: string,
  contextWindow: number,
): Promise<ContextUsage | null> {
  return await invoke<ContextUsage | null>("context_usage", {
    transcriptPath,
    contextWindow,
  });
}

/// 점유 백분율(0~100, 반올림). 측정 불가면 null.
export function contextUsagePct(u: ContextUsage | null): number | null {
  if (!u || u.context_window <= 0) return null;
  return Math.round((u.total_input_tokens / u.context_window) * 100);
}
