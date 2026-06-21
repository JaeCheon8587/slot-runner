import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// 슬롯 PTY IPC 래퍼. 백엔드 src-tauri/src/infra/pty.rs 와 대응.
export type SpawnOpts = {
  cwd?: string | null;
  /** 실행 프로그램(예: "claude"). 미지정 시 OS 셸. */
  program?: string | null;
  /** 프로그램 인자(개별). */
  args?: string[] | null;
};

export const ptySpawn = (id: string, opts: SpawnOpts, cols: number, rows: number) =>
  invoke<void>("pty_spawn", {
    id,
    cwd: opts.cwd ?? null,
    program: opts.program ?? null,
    args: opts.args ?? null,
    cols,
    rows,
  });

export const ptyWrite = (id: string, data: string) =>
  invoke<void>("pty_write", { id, data });

export const ptyResize = (id: string, cols: number, rows: number) =>
  invoke<void>("pty_resize", { id, cols, rows });

export const ptyKill = (id: string) => invoke<void>("pty_kill", { id });

/// 브래킷 페이스트로 주입(멀티라인·슬래시커맨드 안전). 제출(Enter)은 ptyEnter 로 분리.
export const ptyPaste = (id: string, text: string) =>
  ptyWrite(id, `\x1b[200~${text}\x1b[201~`);

export const ptyEnter = (id: string) => ptyWrite(id, "\r");

/// PTY 출력 구독. 바이트(number[])로 받아 Uint8Array 로 변환(멀티바이트 분할 안전).
export function onPtyData(
  cb: (id: string, bytes: Uint8Array) => void,
): Promise<UnlistenFn> {
  return listen<{ id: string; bytes: number[] }>("pty:data", (e) =>
    cb(e.payload.id, new Uint8Array(e.payload.bytes)),
  );
}
