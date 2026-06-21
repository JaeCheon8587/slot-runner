// 슬롯별 로컬 PTY 어댑터 (Infrastructure, docs/ARCHITECTURE.md §4.3 / ADR-002 슬롯=세션).
//
// - 슬롯 id 별 PTY 인스턴스(셸/claude). reader 스레드가 출력을 emit("pty:data") 로 흘린다.
// - 입력은 pty_write(키 주입·파이프라인 슬래시커맨드). 보안: 프로그램·인자 직접 조합 안 함(현재 셸 1개).
// - 슬라이스 2: 셸 spawn + write 배관 증명. claude 기동·운영프롬프트는 후속.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

struct PtyInstance {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[derive(Default)]
pub struct PtyState {
    map: Mutex<HashMap<String, PtyInstance>>,
}

#[derive(Clone, Serialize)]
struct PtyData {
    id: String,
    bytes: Vec<u8>,
}

fn default_shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}

/// 프로그램명을 실행 가능 형태로 해석한다. Windows 에서 npm shim(.cmd/.bat/.ps1)을
/// PATH/PATHEXT 로 찾아 cmd.exe/powershell 래핑한다. 인자는 개별 전달(셸 인젝션 없음).
#[cfg(windows)]
fn resolve_command(program: &str, args: Vec<String>) -> (String, Vec<String>) {
    let has_path_sep = program.contains('/') || program.contains('\\');
    let resolved = if has_path_sep {
        program.to_string()
    } else {
        which_windows(program).unwrap_or_else(|| program.to_string())
    };
    let lower = resolved.to_lowercase();
    if lower.ends_with(".cmd") || lower.ends_with(".bat") {
        let mut a = vec!["/c".to_string(), resolved];
        a.extend(args);
        ("cmd.exe".to_string(), a)
    } else if lower.ends_with(".ps1") {
        let mut a = vec![
            "-NoProfile".to_string(),
            "-ExecutionPolicy".to_string(),
            "Bypass".to_string(),
            "-File".to_string(),
            resolved,
        ];
        a.extend(args);
        ("powershell.exe".to_string(), a)
    } else {
        (resolved, args)
    }
}

#[cfg(windows)]
fn which_windows(name: &str) -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    let exts: Vec<&str> = pathext.split(';').filter(|s| !s.is_empty()).collect();
    for dir in std::env::split_paths(&path) {
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct.to_string_lossy().into_owned());
        }
        for ext in &exts {
            let cand = dir.join(format!("{}{}", name, ext));
            if cand.is_file() {
                return Some(cand.to_string_lossy().into_owned());
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn resolve_command(program: &str, args: Vec<String>) -> (String, Vec<String>) {
    (program.to_string(), args)
}

// Windows Job Object — 슬롯 PTY 자식을 한 job 에 묶는다. KILL_ON_JOB_CLOSE 라서 앱 프로세스가
// 죽으면(정상·크래시·강제kill) 마지막 job 핸들이 닫히며 OS 가 자식 트리(claude→node/dotnet)를 종료한다.
// 고아 프로세스 누적 → 리소스 고갈 → 앱 크래시 악순환을 끊는다.
#[cfg(windows)]
mod jobkill {
    use std::sync::Mutex;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    struct Job(HANDLE);
    // HANDLE 은 프로세스 수명 동안 유지(static). 프로세스 종료 시 OS 가 핸들을 닫아 job 발동.
    unsafe impl Send for Job {}

    static JOB: Mutex<Option<Job>> = Mutex::new(None);

    /// job 을 (최초 1회) 만들고 주어진 pid 를 할당한다. 실패는 무시(고아 정리 best-effort).
    pub fn ensure_and_assign(pid: u32) {
        let mut guard = match JOB.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if guard.is_none() {
            unsafe {
                if let Ok(h) = CreateJobObjectW(None, None) {
                    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    let _ = SetInformationJobObject(
                        h,
                        JobObjectExtendedLimitInformation,
                        &info as *const _ as *const core::ffi::c_void,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    );
                    *guard = Some(Job(h));
                }
            }
        }
        if let Some(job) = guard.as_ref() {
            unsafe {
                if let Ok(hproc) = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid) {
                    let _ = AssignProcessToJobObject(job.0, hproc);
                    let _ = CloseHandle(hproc);
                }
            }
        }
    }
}

/// 슬롯 id 의 PTY 를 spawn 한다. 이미 있으면 에러(중복 spawn 방지).
/// program 지정 시 그 프로그램(예: claude)을, 없으면 OS 셸을 띄운다. args 는 개별 인자(셸 인젝션 없음).
#[tauri::command]
pub fn pty_spawn(
    app: AppHandle,
    state: State<'_, std::sync::Arc<PtyState>>,
    id: String,
    cwd: Option<String>,
    program: Option<String>,
    args: Option<Vec<String>>,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    {
        // 멱등 spawn — 같은 id 가 이미 있으면 기존을 종료·제거 후 새로 띄운다(last spawn wins).
        // dev 이중 마운트·잡 재배정(remount) 시 "이미 존재" 실패와 좀비 세션을 막는다.
        let mut map = state.map.lock().map_err(|_| "state lock 실패")?;
        if let Some(mut old) = map.remove(&id) {
            let _ = old.child.kill();
        }
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| format!("openpty 실패: {}", e))?;

    let prog = program.filter(|s| !s.trim().is_empty()).unwrap_or_else(default_shell);
    // Windows npm shim(claude.cmd 등)은 PATHEXT 해석 + cmd.exe/powershell 래핑 필요
    // (portable-pty 는 CreateProcess 직접 호출이라 PATHEXT 미적용). 인자는 개별 전달(셸 인젝션 없음).
    let (resolved, resolved_args) = resolve_command(&prog, args.unwrap_or_default());
    let mut cmd = CommandBuilder::new(resolved);
    for a in resolved_args {
        cmd.arg(a);
    }
    if let Some(d) = cwd.filter(|s| !s.trim().is_empty()) {
        cmd.cwd(d);
    }
    // 슬롯 식별 환경변수(후속: Hook panel_id 라우팅).
    cmd.env("SLOTRUNNER_PANEL_ID", &id);

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("spawn 실패: {}", e))?;
    // Windows: 자식을 Job Object 에 묶어 앱 종료(크래시 포함) 시 OS 가 트리째 종료 → 고아 방지.
    #[cfg(windows)]
    if let Some(pid) = child.process_id() {
        jobkill::ensure_and_assign(pid);
    }
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("reader clone 실패: {}", e))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("writer 획득 실패: {}", e))?;

    // 출력 펌프 스레드: PTY → emit("pty:data"). 바이트로 보내 멀티바이트 분할 손상 회피(프론트 Uint8Array).
    let app2 = app.clone();
    let id2 = id.clone();
    std::thread::Builder::new()
        .name(format!("pty-read-{}", id))
        .spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let _ = app2.emit(
                            "pty:data",
                            PtyData { id: id2.clone(), bytes: buf[..n].to_vec() },
                        );
                    }
                }
            }
            let _ = app2.emit("pty:exit", id2.clone());
        })
        .map_err(|e| format!("reader 스레드 실패: {}", e))?;

    state
        .map
        .lock()
        .map_err(|_| "state lock 실패")?
        .insert(id, PtyInstance { master: pair.master, writer, child });
    Ok(())
}

/// PTY 에 입력 전달(키 입력·주입). 외부 텍스트는 데이터로만 — 명령 조합 안 함.
#[tauri::command]
pub fn pty_write(
    state: State<'_, std::sync::Arc<PtyState>>,
    id: String,
    data: String,
) -> Result<(), String> {
    let mut map = state.map.lock().map_err(|_| "state lock 실패")?;
    let inst = map.get_mut(&id).ok_or_else(|| format!("pty 없음: {}", id))?;
    inst.writer.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    inst.writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn pty_resize(
    state: State<'_, std::sync::Arc<PtyState>>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let map = state.map.lock().map_err(|_| "state lock 실패")?;
    let inst = map.get(&id).ok_or_else(|| format!("pty 없음: {}", id))?;
    inst.master
        .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_kill(
    state: State<'_, std::sync::Arc<PtyState>>,
    id: String,
) -> Result<(), String> {
    let mut map = state.map.lock().map_err(|_| "state lock 실패")?;
    if let Some(mut inst) = map.remove(&id) {
        let _ = inst.child.kill();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// portable-pty spawn → write(echo) → read 라운드트립. 슬롯 PTY 주입 배관의 핵심 메커니즘 검증
    /// (Tauri/xterm 비의존). pty_spawn/pty_write 커맨드는 이 메커니즘의 얇은 래퍼.
    #[test]
    fn pty_spawn_write_read_roundtrip() {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .expect("openpty");
        let mut child = pair
            .slave
            .spawn_command(CommandBuilder::new(default_shell()))
            .expect("spawn");
        let mut reader = pair.master.try_clone_reader().expect("reader");
        let mut writer = pair.master.take_writer().expect("writer");

        let acc = Arc::new(Mutex::new(String::new()));
        let acc2 = acc.clone();
        let reader_thread = std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => acc2
                        .lock()
                        .unwrap()
                        .push_str(&String::from_utf8_lossy(&buf[..n])),
                }
            }
        });

        writer.write_all(b"echo SLOTRUNNER_PTY_OK\r\n").expect("write");
        writer.flush().expect("flush");

        let start = Instant::now();
        let mut seen = false;
        while start.elapsed() < Duration::from_secs(8) {
            if acc.lock().unwrap().contains("SLOTRUNNER_PTY_OK") {
                seen = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // 정리: 자식 종료 + 쓰기/마스터 drop(reader EOF 유도). reader 스레드는 join 하지 않는다
        // — ConPTY 에서 kill 후 read 가 즉시 EOF 안 날 수 있어 join 시 무한 블로킹(테스트 hang) 위험.
        let _ = child.kill();
        drop(writer);
        drop(pair.master);
        let _ = reader_thread; // detach

        let captured = acc.lock().unwrap().clone();
        assert!(seen, "PTY echo 마커 미수신. 누적 출력:\n{}", captured);
    }
}
