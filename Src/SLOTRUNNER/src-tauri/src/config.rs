// 앱 설정 (SLOTRUNNER-PRD §8 정량값 · SPEC §5). OS 표준 위치(app_config_dir)의 config.json.
// 자격증명 미저장 — 포트/슬롯수/주입지연뿐. 알 수 없는 필드 무시(serde default).

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// 봇 인테이크 REST 포트 (ADR-001, 127.0.0.1).
    pub rest_port: u16,
    /// 동시 슬롯 수 (기본 2, 상한 4 — ADR-002).
    pub slots: u8,
    /// 슬롯 claude 기동 후 forge 주입까지 정착 지연(ms). 휴리스틱(후속 Hook SessionStart 로 대체).
    pub inject_delay_ms: u32,
    /// 스텝 전이 시 컨텍스트 점유율(%)이 이 값 이상이면 다음 스텝 전에 /compact 주입(sidabari 참고).
    /// 기본 40. 0/100 이면 사실상 항상/거의-안-함. 측정 불가(transcript 없음)면 보수적으로 압축.
    pub compact_threshold_pct: u8,
    /// 컨텍스트 윈도우 토큰(점유율 분모). 기본 1,000,000 (Claude Code 1M 컨텍스트).
    pub context_window_tokens: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            rest_port: 8765,
            slots: 2,
            inject_delay_ms: 4000,
            compact_threshold_pct: 40,
            context_window_tokens: 1_000_000,
        }
    }
}

impl AppConfig {
    /// 범위 보정 — slots 1..=4 (리소스 상한), 포트 0 방지, 임계 0..=100, 윈도우 0 방지.
    pub fn normalized(mut self) -> Self {
        self.slots = self.slots.clamp(1, 4);
        if self.rest_port == 0 {
            self.rest_port = 8765;
        }
        self.compact_threshold_pct = self.compact_threshold_pct.min(100);
        if self.context_window_tokens == 0 {
            self.context_window_tokens = 1_000_000;
        }
        self
    }
}

/// config.json 로드. 부재/파싱 실패 시 기본값(앱은 계속 동작).
pub fn load(app: &AppHandle) -> AppConfig {
    let path = match app.path().app_config_dir() {
        Ok(d) => d.join("config.json"),
        Err(_) => return AppConfig::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str::<AppConfig>(&s).unwrap_or_default().normalized(),
        Err(_) => AppConfig::default().normalized(),
    }
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> AppConfig {
    load(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = AppConfig::default();
        assert_eq!(c.rest_port, 8765);
        assert_eq!(c.slots, 2);
        assert_eq!(c.inject_delay_ms, 4000);
        assert_eq!(c.compact_threshold_pct, 40);
        assert_eq!(c.context_window_tokens, 1_000_000);
    }

    #[test]
    fn partial_json_fills_defaults() {
        // 일부 필드만 → 나머지 default (serde default).
        let c: AppConfig = serde_json::from_str(r#"{"slots":3}"#).unwrap();
        assert_eq!(c.slots, 3);
        assert_eq!(c.rest_port, 8765);
    }

    #[test]
    fn normalize_clamps_slots() {
        let c = AppConfig {
            rest_port: 0,
            slots: 9,
            inject_delay_ms: 1,
            compact_threshold_pct: 200,
            context_window_tokens: 0,
        }
        .normalized();
        assert_eq!(c.slots, 4);
        assert_eq!(c.rest_port, 8765);
        assert_eq!(c.compact_threshold_pct, 100); // clamp
        assert_eq!(c.context_window_tokens, 1_000_000); // 0 방지
    }
}
