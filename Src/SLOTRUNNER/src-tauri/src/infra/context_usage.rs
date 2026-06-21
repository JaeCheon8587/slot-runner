// 슬롯 claude 세션의 "현재 컨텍스트 점유"를 트랜스크립트(.jsonl)에서 추정한다 (Infrastructure).
// sidabari4loop supervisor.rs 의 context_usage 를 SlotRunner 스텝 루프용으로 적응.
//
// 현재 점유 ≈ 마지막 assistant 메시지의 (input + cache_creation + cache_read) input 토큰.
//   (output_tokens 는 생성 응답이라 컨텍스트 누적분이 아님.)
// 경로는 Hook payload 의 transcript_path 에서 오므로 임의 파일 읽기를 막는다 (sidabari §1.2.2):
//   절대경로 + 확장자 .jsonl + 경로에 `.claude/projects` 컴포넌트 연속 포함.
// 파일이 없거나 usage 를 못 찾으면 Ok(None) (프론트는 "측정불가"로 보수적 압축).

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, PathBuf};

use serde::Serialize;
use serde_json::Value;

/// context_window 가 0/미지정일 때의 폴백.
const DEFAULT_CONTEXT_WINDOW: u64 = 1_000_000;
/// 트랜스크립트는 길어질 수 있어 끝부분만 읽는다.
const TRANSCRIPT_TAIL_BYTES: u64 = 512 * 1024;

#[derive(Debug, Serialize)]
pub struct ContextUsage {
    /// input + cache_creation + cache_read (현재 프롬프트에 들어간 컨텍스트 토큰 합).
    pub total_input_tokens: u64,
    /// 마지막 assistant 메시지의 생성 토큰 (참고용 — 컨텍스트 누적 아님).
    pub output_tokens: u64,
    /// 분모(컨텍스트 윈도우).
    pub context_window: u64,
}

/// transcript_path 검증 — 절대경로 + .jsonl + `.claude/projects` 하위. 위반 시 Err.
fn validate_transcript_path(path: &PathBuf) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("transcript_path는 절대경로여야 합니다".to_string());
    }
    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
        return Err("transcript_path는 .jsonl 파일이어야 합니다".to_string());
    }
    let comps: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str().map(|s| s.to_string()),
            _ => None,
        })
        .collect();
    let under_claude_projects = comps
        .windows(2)
        .any(|w| w[0] == ".claude" && w[1] == "projects");
    if !under_claude_projects {
        return Err("transcript_path가 .claude/projects 하위가 아닙니다".to_string());
    }
    Ok(())
}

/// transcript 끝부분에서 마지막 (비 sidechain) assistant usage 를 찾아 점유 토큰을 추정.
#[tauri::command]
pub async fn context_usage(
    transcript_path: String,
    context_window: u64,
) -> Result<Option<ContextUsage>, String> {
    let window = if context_window > 0 {
        context_window
    } else {
        DEFAULT_CONTEXT_WINDOW
    };
    let path = PathBuf::from(transcript_path.trim());
    validate_transcript_path(&path)?;
    if !path.is_file() {
        return Ok(None);
    }

    let mut file =
        fs::File::open(&path).map_err(|e| format!("트랜스크립트 열기 실패: {}", e))?;
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(TRANSCRIPT_TAIL_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|e| format!("트랜스크립트 seek 실패: {}", e))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("트랜스크립트 읽기 실패: {}", e))?;
    // 끝에서 잘라 읽어 첫 줄은 깨졌을 수 있다 → lossy + 거꾸로 스캔하며 파싱 실패 줄 skip.
    let text = String::from_utf8_lossy(&bytes);

    for line in text.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // 서브에이전트(sidechain) 메시지는 메인 컨텍스트가 아니므로 제외.
        if v.get("isSidechain").and_then(|b| b.as_bool()) == Some(true) {
            continue;
        }
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let usage = match v.get("message").and_then(|m| m.get("usage")) {
            Some(u) => u,
            None => continue,
        };
        let field = |k: &str| usage.get(k).and_then(|n| n.as_u64()).unwrap_or(0);
        let total_input_tokens = field("input_tokens")
            + field("cache_creation_input_tokens")
            + field("cache_read_input_tokens");
        return Ok(Some(ContextUsage {
            total_input_tokens,
            output_tokens: field("output_tokens"),
            context_window: window,
        }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_jsonl() {
        let p = PathBuf::from("C:/x/.claude/projects/foo/t.txt");
        assert!(validate_transcript_path(&p).is_err());
    }

    #[test]
    fn rejects_outside_claude_projects() {
        let p = PathBuf::from("C:/x/y/t.jsonl");
        assert!(validate_transcript_path(&p).is_err());
    }

    #[test]
    fn accepts_valid_transcript_path() {
        let p = PathBuf::from("C:/Users/me/.claude/projects/repo-slug/abc.jsonl");
        assert!(validate_transcript_path(&p).is_ok());
    }

    #[test]
    fn rejects_relative_path() {
        let p = PathBuf::from(".claude/projects/x/t.jsonl");
        assert!(validate_transcript_path(&p).is_err());
    }
}
