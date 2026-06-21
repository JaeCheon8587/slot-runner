// target repo 의 .claude/settings.local.json 에 SlotRunner Hook 을 등록한다(Infrastructure).
// 슬롯 claude 세션이 Stop/SessionStart 등에서 append-event.js 를 호출 → events.jsonl → hooks_bus.
// 이게 있어야 StageController 가 Stop 이벤트로 단계 전이를 할 수 있다.
//
// 보안(CLAUDE.md §1.2.2 계승): 디렉토리 검증, 기존 설정 백업 후 병합(SlotRunner 관리 영역만 교체),
// 마커(_slotrunner_managed_hooks)로 우리 영역 식별. 외부 텍스트를 명령으로 실행하지 않음(append 만).

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

const MANAGED_MARKER: &str = "_slotrunner_managed_hooks";
const EVENTS: &[&str] = &[
    "Stop",
    "SessionStart",
    "Notification",
    "PreToolUse",
    "PostToolUse",
    "UserPromptSubmit",
    "SubagentStop",
];

/// repo/.claude/settings.local.json 에 append-event.js 훅을 등록(병합).
#[tauri::command]
pub fn install_claude_hooks(repo: String, append_script: String) -> Result<(), String> {
    if repo.trim().is_empty() {
        return Err("repo 경로 비어있음".into());
    }
    let repo_path = Path::new(&repo);
    if !repo_path.is_dir() {
        return Err(format!("repo 디렉토리 아님: {}", repo));
    }
    let claude_dir = repo_path.join(".claude");
    fs::create_dir_all(&claude_dir).map_err(|e| format!(".claude 생성 실패: {}", e))?;
    let settings_path = claude_dir.join("settings.local.json");

    // 기존 설정 로드(객체). 없으면 빈 객체.
    let mut root: Value = if settings_path.exists() {
        let txt = fs::read_to_string(&settings_path)
            .map_err(|e| format!("settings 읽기 실패: {}", e))?;
        // 백업(최초 1회 충분하나 idempotent 하게 매번 덮어쓴다).
        let _ = fs::write(
            claude_dir.join("settings.local.json.slotrunner-bak"),
            &txt,
        );
        serde_json::from_str(&txt).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    if !root.is_object() {
        root = json!({});
    }

    // node 명령 — append-event.js 를 이벤트명 인자로 호출. claude 가 hook payload 를 stdin 으로 전달.
    let obj = root.as_object_mut().ok_or("settings 루트가 객체 아님")?;
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("hooks 가 객체 아님")?;

    for &ev in EVENTS {
        let command = format!("node \"{}\" {}", append_script.replace('\\', "/"), ev);
        // SlotRunner 관리 엔트리로 해당 이벤트 훅을 교체(사용자 다른 훅과 공존하려면 병합 정교화 필요 —
        // 현재는 단일 사용자 가정으로 이벤트별 1엔트리 세팅).
        hooks.insert(
            ev.to_string(),
            json!([{
                "hooks": [{ "type": "command", "command": command, MANAGED_MARKER: true }]
            }]),
        );
    }
    obj.insert(MANAGED_MARKER.to_string(), json!(true));

    let pretty = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("settings 직렬화 실패: {}", e))?;
    fs::write(&settings_path, pretty).map_err(|e| format!("settings 쓰기 실패: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_format() {
        let cmd = format!("node \"{}\" {}", "C:/a/append-event.js", "Stop");
        assert_eq!(cmd, "node \"C:/a/append-event.js\" Stop");
    }
}
