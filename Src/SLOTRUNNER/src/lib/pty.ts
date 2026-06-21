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

/// 브래킷 페이스트로 주입(멀티라인 프롬프트 안전 — 조기 제출 방지). 제출(Enter)은 ptyEnter 로 분리.
export const ptyPaste = (id: string, text: string) =>
  ptyWrite(id, `\x1b[200~${text}\x1b[201~`);

export const ptyEnter = (id: string) => ptyWrite(id, "\r");

const ESC = "\x1b";
function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

/// 슬래시 명령(/compact 등) 1개를 제출한다 (sidabari injectSlashCommand 이식).
/// 브래킷 페이스트(ptyPaste)는 "붙여넣기 텍스트"라 슬래시 메뉴가 안 떠 리터럴로 박힐 수 있다.
/// 그래서: (1) 턴 종료 직후 입력창 준비 지연 → (2) ESC 로 잔여 입력/열린 메뉴 정리 →
/// (3) 타이핑으로 명령 입력(슬래시 메뉴 발동) → (4) 메뉴 인식 지연 → (5) Enter.
export async function ptySlash(id: string, command: string): Promise<void> {
  await delay(400); // 입력 프롬프트 준비
  await ptyWrite(id, ESC); // 잔여 입력/메뉴 정리(재시도 안전)
  await delay(80);
  await ptyWrite(id, command); // 페이스트 아닌 타이핑 → 슬래시 메뉴 발동
  await delay(250); // 메뉴가 명령 인식할 시간
  await ptyWrite(id, "\r");
}

/// PTY 출력 구독. 바이트(number[])로 받아 Uint8Array 로 변환(멀티바이트 분할 안전).
export function onPtyData(
  cb: (id: string, bytes: Uint8Array) => void,
): Promise<UnlistenFn> {
  return listen<{ id: string; bytes: number[] }>("pty:data", (e) =>
    cb(e.payload.id, new Uint8Array(e.payload.bytes)),
  );
}
