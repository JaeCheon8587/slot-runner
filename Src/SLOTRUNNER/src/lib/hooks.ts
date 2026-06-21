import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// Claude Hook 이벤트 (백엔드 hooks_bus.rs). kind = stop|session-start|notification|pretool|posttool|...
export type HookEvent = {
  kind: string;
  panel_id: string | null;
  payload: Record<string, unknown>;
};

export function listenHookEvent(cb: (e: HookEvent) => void): Promise<UnlistenFn> {
  return listen<HookEvent>("hook:event", (e) => cb(e.payload));
}

export type HookPaths = { base_dir: string; append_script: string };
export const hookPaths = () => invoke<HookPaths>("hook_paths");

/// target repo .claude/settings.local.json 에 SlotRunner 훅 등록(스텝 루프 Stop 이벤트용).
export const installClaudeHooks = (repo: string, appendScript: string) =>
  invoke<void>("install_claude_hooks", { repo, appendScript });
