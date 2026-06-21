// SlotRunner — Presentation 조립 루트 (Tauri).
// 레이어 규칙: docs/ARCHITECTURE.md §4.4 — 본 파일은 조립·DI·생명주기만. 정책은 app/, IO 는 infra/.
mod config;
mod domain;
mod infra;

use std::sync::Arc;

use tauri::Manager;

use infra::pty::PtyState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(Arc::new(PtyState::default()))
        .setup(|app| {
            // 설정 로드(포트 등). 부재 시 기본값.
            let cfg = config::load(app.handle());
            // 봇→앱 동기 REST 인테이크 시작 (ADR-001, 127.0.0.1 바인드).
            // 실패해도 앱은 계속 동작(인테이크만 비활성) — sidabari §1.3 자동 재시도 금지 정책 계승.
            let handle = app.handle().clone();
            if let Err(e) = infra::rest_server::start(handle, cfg.rest_port) {
                eprintln!("[lib] REST 시작 실패 — 잡 인테이크 비활성: {}", e);
            }
            // Claude Hook 이벤트 버스. 실패해도 앱 계속(훅 기능만 비활성).
            match infra::hooks_bus::init(app.handle()) {
                Ok(bus) => {
                    app.manage(Arc::new(bus));
                }
                Err(e) => eprintln!("[lib] hooks_bus init 실패 — 훅 비활성: {}", e),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::get_config,
            infra::pty::pty_spawn,
            infra::pty::pty_write,
            infra::pty::pty_resize,
            infra::pty::pty_kill,
            infra::gate::read_forge_gate,
            infra::gate::read_ddr_gate,
            infra::context_usage::context_usage,
            infra::hooks_bus::hook_paths,
            infra::hook_installer::install_claude_hooks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
