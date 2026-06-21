// 단계 게이트 — forge index.json / ddr .review 를 읽어 결정적으로 판정(ADR-003).
// LLM 자기보고 비의존. develop-small.py `_phase_status`/`_ddr_review_path` 계승.
//
// forge index.json 위치: <repo>/.worktrees/<phase>/phases/scoped/<phase>/index.json
//   steps[].status 가 전부 "completed" → "ok", 아니면 첫 비-completed status.
// ddr review 위치: <repo>/.worktrees/<phase>/.review/<stem>-review.md (없으면 <repo>/.review/...)

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize, PartialEq)]
pub struct StepStatus {
    pub step: u64,
    pub status: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ForgeGate {
    /// "ok" | "blocked" | "error" | "pending" | "interrupted" | "empty" | "missing" | "invalid" | ...
    pub status: String,
    pub total: usize,
    pub completed: usize,
    pub steps: Vec<StepStatus>,
}

/// phase 슬러그 검증 — 경로 traversal 방어(영숫자·하이픈만).
fn valid_phase(phase: &str) -> bool {
    !phase.is_empty()
        && phase.len() <= 80
        && phase.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn index_path(repo: &str, phase: &str) -> PathBuf {
    Path::new(repo)
        .join(".worktrees")
        .join(phase)
        .join("phases")
        .join("scoped")
        .join(phase)
        .join("index.json")
}

/// index.json 문자열을 판정(순수). 모든 step completed → "ok", 아니면 첫 비-completed status.
pub fn evaluate_index(data: &str) -> ForgeGate {
    let v: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(_) => return ForgeGate { status: "invalid".into(), total: 0, completed: 0, steps: vec![] },
    };
    let arr = match v.get("steps").and_then(|s| s.as_array()) {
        Some(a) => a,
        None => return ForgeGate { status: "empty".into(), total: 0, completed: 0, steps: vec![] },
    };
    let mut steps: Vec<StepStatus> = arr
        .iter()
        .map(|s| StepStatus {
            step: s.get("step").and_then(|x| x.as_u64()).unwrap_or(0),
            status: s
                .get("status")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string(),
        })
        .collect();
    steps.sort_by_key(|s| s.step);

    if steps.is_empty() {
        return ForgeGate { status: "empty".into(), total: 0, completed: 0, steps };
    }
    let completed = steps.iter().filter(|s| s.status == "completed").count();
    let bad = steps.iter().find(|s| s.status != "completed");
    let status = match bad {
        None => "ok".to_string(),
        Some(s) => s.status.clone(),
    };
    ForgeGate { status, total: steps.len(), completed, steps }
}

/// forge 게이트 판정. index.json 부재 시 status="missing".
#[tauri::command]
pub fn read_forge_gate(repo: String, phase: String) -> Result<ForgeGate, String> {
    if !valid_phase(&phase) {
        return Err(format!("부정한 phase 슬러그: {}", phase));
    }
    let p = index_path(&repo, &phase);
    match fs::read_to_string(&p) {
        Ok(s) => Ok(evaluate_index(&s)),
        Err(_) => Ok(ForgeGate { status: "missing".into(), total: 0, completed: 0, steps: vec![] }),
    }
}

/// ddr 리뷰 산출물 존재 여부(비어있지 않은 파일). 워크트리 우선, 없으면 메인 repo.
#[tauri::command]
pub fn read_ddr_gate(repo: String, phase: String, stem: String) -> Result<bool, String> {
    if !valid_phase(&phase) {
        return Err(format!("부정한 phase 슬러그: {}", phase));
    }
    let file = format!("{}-review.md", stem);
    let candidates = [
        Path::new(&repo).join(".worktrees").join(&phase).join(".review").join(&file),
        Path::new(&repo).join(".review").join(&file),
    ];
    for c in candidates {
        if let Ok(meta) = fs::metadata(&c) {
            if meta.is_file() && meta.len() > 0 {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_completed_is_ok() {
        let j = r#"{"steps":[{"step":1,"status":"completed"},{"step":0,"status":"completed"}]}"#;
        let g = evaluate_index(j);
        assert_eq!(g.status, "ok");
        assert_eq!(g.total, 2);
        assert_eq!(g.completed, 2);
        assert_eq!(g.steps[0].step, 0); // 정렬됨
    }

    #[test]
    fn first_bad_status_wins() {
        let j = r#"{"steps":[{"step":0,"status":"completed"},{"step":1,"status":"blocked"},{"step":2,"status":"pending"}]}"#;
        let g = evaluate_index(j);
        assert_eq!(g.status, "blocked");
        assert_eq!(g.completed, 1);
        assert_eq!(g.total, 3);
    }

    #[test]
    fn no_steps_is_empty() {
        assert_eq!(evaluate_index(r#"{"steps":[]}"#).status, "empty");
        assert_eq!(evaluate_index(r#"{}"#).status, "empty");
    }

    #[test]
    fn bad_json_is_invalid() {
        assert_eq!(evaluate_index("not json").status, "invalid");
    }

    #[test]
    fn phase_slug_guard() {
        assert!(valid_phase("master-task-014"));
        assert!(!valid_phase("../etc"));
        assert!(!valid_phase("a/b"));
        assert!(!valid_phase(""));
    }
}
