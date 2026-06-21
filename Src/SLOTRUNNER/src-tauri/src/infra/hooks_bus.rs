// Claude Code Hook 이벤트 수신 (Infrastructure). 파일 기반 IPC:
//   <app_data>/slotrunner-hooks/
//     scripts/append-event.js  (claude 훅이 호출 → events.jsonl append)
//     events.jsonl             (append-only, notify watcher 가 tail)
// watcher 가 새 줄을 읽어 classify_event 후 emit("hook:event", {kind, panel_id, payload}).
// 프론트 HookBridge 가 panel_id(=슬롯)로 라우팅 → Stop 시 게이트 자동 판정 등.
// 보안: 외부 텍스트(payload)는 데이터일 뿐, 명령 실행 안 함(sidabari §1.2 계승).

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

const HOOKS_SUBDIR: &str = "slotrunner-hooks";
const APPEND_SCRIPT_BODY: &str = include_str!("../../resources/append-event.js");

#[derive(Debug, Clone, Serialize)]
pub struct HookPaths {
    pub base_dir: String,
    pub append_script: String,
}

pub struct HookBusState {
    pub paths: HookPaths,
    _watcher: Mutex<RecommendedWatcher>,
}

#[derive(Debug, Serialize, Clone)]
struct HookEventEmit {
    kind: String,
    panel_id: Option<String>,
    payload: Value,
}

/// claude 훅 이벤트명을 내부 kind 로 분류(순수).
fn classify_event(payload: &Value) -> String {
    let raw = payload
        .get("_slotrunner")
        .and_then(|s| s.get("hook_event_name_arg"))
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("hook_event_name").and_then(|v| v.as_str()))
        .unwrap_or("unknown");
    match raw {
        "Stop" => "stop",
        "SessionStart" => "session-start",
        "Notification" => "notification",
        "PreToolUse" => "pretool",
        "PostToolUse" => "posttool",
        "SubagentStop" => "subagent-stop",
        "UserPromptSubmit" => "user-prompt",
        other => return format!("other:{}", other),
    }
    .to_string()
}

fn panel_id_of(payload: &Value) -> Option<String> {
    payload
        .get("_slotrunner")
        .and_then(|s| s.get("panel_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn base_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir 실패: {}", e))?;
    Ok(dir.join(HOOKS_SUBDIR))
}

/// setup 에서 호출. 디렉토리·스크립트 배포 + events.jsonl + watcher 시작.
pub fn init(app: &AppHandle) -> Result<HookBusState, String> {
    let base = base_dir(app)?;
    let scripts_dir = base.join("scripts");
    fs::create_dir_all(&scripts_dir)
        .map_err(|e| format!("hooks 디렉토리 생성 실패: {}", e))?;

    let append_path = scripts_dir.join("append-event.js");
    fs::write(&append_path, APPEND_SCRIPT_BODY)
        .map_err(|e| format!("append-event.js 쓰기 실패: {}", e))?;

    let events_path = base.join("events.jsonl");
    if !events_path.exists() {
        fs::write(&events_path, b"").map_err(|e| format!("events.jsonl 생성 실패: {}", e))?;
    }
    // 시작 시점 파일 끝을 기준 offset(과거 이벤트 무시).
    let start_offset = events_path.metadata().map(|m| m.len()).unwrap_or(0);

    let paths = HookPaths {
        base_dir: base.to_string_lossy().into_owned(),
        append_script: append_path.to_string_lossy().into_owned(),
    };

    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .map_err(|e| format!("watcher 생성 실패: {}", e))?;
    watcher
        .watch(&base, RecursiveMode::NonRecursive)
        .map_err(|e| format!("watcher 등록 실패: {}", e))?;

    {
        let app = app.clone();
        let events_path = events_path.clone();
        let mut offset = start_offset;
        std::thread::spawn(move || {
            for ev in rx {
                let event = match ev {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("[hooks_bus] watch error: {}", e);
                        continue;
                    }
                };
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))
                    && event.paths.iter().any(|p| p == &events_path)
                {
                    tail_events(&app, &events_path, &mut offset);
                }
            }
        });
    }

    Ok(HookBusState { paths, _watcher: Mutex::new(watcher) })
}

fn tail_events(app: &AppHandle, events_path: &Path, offset: &mut u64) {
    let mut file = match File::open(events_path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    if len < *offset {
        *offset = 0; // truncate/rotate
    }
    if len == *offset {
        return;
    }
    if file.seek(SeekFrom::Start(*offset)).is_err() {
        return;
    }
    let reader = BufReader::new(file);
    let mut new_offset = *offset;
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        new_offset += line.len() as u64 + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<Value>(trimmed) {
            let kind = classify_event(&payload);
            let panel_id = panel_id_of(&payload);
            let _ = app.emit("hook:event", &HookEventEmit { kind, panel_id, payload });
        }
    }
    *offset = new_offset;
}

#[tauri::command]
pub fn hook_paths(state: tauri::State<'_, std::sync::Arc<HookBusState>>) -> HookPaths {
    state.paths.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_known_events() {
        let p = json!({"_slotrunner": {"hook_event_name_arg": "Stop"}});
        assert_eq!(classify_event(&p), "stop");
        let p = json!({"_slotrunner": {"hook_event_name_arg": "SessionStart"}});
        assert_eq!(classify_event(&p), "session-start");
    }

    #[test]
    fn classify_direct_field_fallback() {
        let p = json!({"hook_event_name": "PostToolUse"});
        assert_eq!(classify_event(&p), "posttool");
    }

    #[test]
    fn classify_unknown_prefixed() {
        let p = json!({"_slotrunner": {"hook_event_name_arg": "Weird"}});
        assert_eq!(classify_event(&p), "other:Weird");
    }

    #[test]
    fn panel_id_extracted() {
        let p = json!({"_slotrunner": {"panel_id": "slot-2"}});
        assert_eq!(panel_id_of(&p), Some("slot-2".to_string()));
        assert_eq!(panel_id_of(&json!({})), None);
    }
}
