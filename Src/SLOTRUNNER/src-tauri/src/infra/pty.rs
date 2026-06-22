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
    // .PS1 보강(npm claude.ps1 등). 기본값은 Windows 표준 + .PS1.
    let pathext =
        std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD;.PS1".to_string());
    let exts: Vec<String> = pathext
        .split(';')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let dirs: Vec<std::path::PathBuf> = std::env::split_paths(&path).collect();
    resolve_in_dirs(name, &dirs, &exts)
}

/// PATH 디렉토리들에서 실행 파일을 해석한다(순수 — env 비의존, 테스트 가능).
/// 확장자 없는 이름은 **PATHEXT 변형만** 매칭한다 — 확장자 없는 bare 파일
/// (npm Unix shim `claude` = 셸 스크립트)은 CreateProcessW 가 실행 못 해
/// os error 193(BAD_EXE_FORMAT) 이 나므로 절대 반환하지 않는다.
/// 이름에 이미 확장자가 있으면(예: `claude.cmd`) 그 파일만 찾는다.
#[cfg(windows)]
fn resolve_in_dirs(name: &str, dirs: &[std::path::PathBuf], exts: &[String]) -> Option<String> {
    let has_ext = std::path::Path::new(name).extension().is_some();
    for dir in dirs {
        if has_ext {
            let direct = dir.join(name);
            if direct.is_file() {
                return Some(direct.to_string_lossy().into_owned());
            }
            continue;
        }
        for ext in exts {
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

// Windows Job Object — 슬롯마다 별도 job 을 만들어 그 슬롯 PTY 자식(과 자손 트리)을 묶는다.
// KILL_ON_JOB_CLOSE 라서 해당 슬롯 job 핸들이 닫히면 OS 가 자식 트리(claude→node/dotnet)를 종료한다.
// - 슬롯 해제(pty_kill)·재기동(remount): 그 슬롯 job 핸들만 닫아 → 서브트리 즉시 reap (자손까지).
//   child.kill() 은 최상위 1개만 죽여 자손(node/dotnet)이 고아로 남던 문제를 막는다.
// - 앱 종료/크래시: 프로세스 종료 시 OS 가 남은 모든 슬롯 job 핸들을 닫아 → 전체 트리 reap (백스톱).
// 고아 프로세스 누적 → 리소스 고갈 → 앱 크래시 악순환을 끊는다.
#[cfg(windows)]
mod jobkill {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    struct Job(HANDLE);
    // HANDLE 은 슬롯 수명 동안 유지. 닫는 순간(close/remount/프로세스 종료) KILL_ON_JOB_CLOSE 발동.
    unsafe impl Send for Job {}

    static JOBS: Mutex<Option<HashMap<String, Job>>> = Mutex::new(None);

    /// 슬롯 id 전용 job 을 만들어 pid 를 할당한다. 같은 id 의 기존 job 이 있으면 그 핸들을 닫아
    /// (이전 슬롯 서브트리 reap) 새 job 으로 교체한다. 실패는 무시(고아 정리 best-effort).
    pub fn assign(id: &str, pid: u32) {
        let mut guard = match JOBS.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let map = guard.get_or_insert_with(HashMap::new);
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
                if let Ok(hproc) = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid) {
                    let _ = AssignProcessToJobObject(h, hproc);
                    let _ = CloseHandle(hproc);
                }
                // 같은 id 의 이전 job 교체 — 반환된 옛 핸들을 닫아 이전 서브트리 reap.
                if let Some(old) = map.insert(id.to_string(), Job(h)) {
                    let _ = CloseHandle(old.0);
                }
            }
        }
    }

    /// 슬롯 id 의 job 핸들을 닫는다 → KILL_ON_JOB_CLOSE 로 그 슬롯 서브트리(자손 포함) 즉시 종료.
    pub fn close(id: &str) {
        if let Ok(mut guard) = JOBS.lock() {
            if let Some(map) = guard.as_mut() {
                if let Some(job) = map.remove(id) {
                    unsafe {
                        let _ = CloseHandle(job.0);
                    }
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
            // 이전 슬롯 job 닫아 자손 트리까지 reap (child.kill 은 최상위만 죽임).
            #[cfg(windows)]
            jobkill::close(&id);
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
    // Windows: 자식을 슬롯 전용 Job Object 에 묶는다. 슬롯 해제(pty_kill)·재기동 시 그 슬롯 job 만
    // 닫아 자손 트리째 reap, 앱 종료(크래시 포함) 시 모든 슬롯 job 핸들이 닫혀 전체 종료 → 고아 방지.
    #[cfg(windows)]
    if let Some(pid) = child.process_id() {
        jobkill::assign(&id, pid);
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
        // 슬롯 job 핸들 닫아 자손 트리(node/dotnet)까지 즉시 reap — 슬롯 해제 시 고아 방지.
        #[cfg(windows)]
        jobkill::close(&id);
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

    /// 확장자 없는 `claude`(npm Unix shim, 셸 스크립트)가 아니라 `claude.cmd`(PATHEXT)를
    /// 골라야 한다 — bare 매칭은 CreateProcessW os error 193(회사 환경 버그)을 낸다.
    #[cfg(windows)]
    #[test]
    fn which_prefers_pathext_over_extensionless_shim() {
        use std::fs;
        let base = std::env::temp_dir().join(format!("slotrunner_which_{}", std::process::id()));
        let _ = fs::create_dir_all(&base);
        // npm 처럼 확장자 없는 shim + .cmd 둘 다 존재시킨다.
        fs::write(base.join("claude"), b"#!/bin/sh\necho shim").unwrap();
        fs::write(base.join("claude.cmd"), b"@echo off\r\n").unwrap();
        let exts: Vec<String> = vec![".EXE".to_string(), ".CMD".to_string()];

        // 확장자 없는 이름 → bare shim 무시, claude.cmd 선택.
        let got = resolve_in_dirs("claude", std::slice::from_ref(&base), &exts).expect("resolve");
        assert!(
            got.to_lowercase().ends_with("claude.cmd"),
            "확장자 없는 shim 을 잘못 매칭: {}",
            got
        );
        // 명시 확장자 → 그 파일.
        let got2 =
            resolve_in_dirs("claude.cmd", std::slice::from_ref(&base), &exts).expect("resolve2");
        assert!(got2.to_lowercase().ends_with("claude.cmd"));

        let _ = fs::remove_dir_all(&base);
    }

    /// 슬롯 job close() 가 할당된 프로세스 트리를 reap 하는지 검증 (슬롯 해제 시 고아 방지의 핵심).
    /// cmd→ping 2단계 트리를 슬롯 job 에 묶고 close → KILL_ON_JOB_CLOSE 로 종료되는지 확인.
    /// (자손 reap 은 job 멤버십 상속 + KILL_ON_JOB_CLOSE 의 OS 보장 — 앱 kill 라이브 테스트에서도 관측됨.)
    #[cfg(windows)]
    #[test]
    fn jobkill_close_reaps_assigned_tree() {
        use std::process::Command;
        fn alive(pid: u32) -> bool {
            let out = Command::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"])
                .output()
                .expect("tasklist");
            String::from_utf8_lossy(&out.stdout).contains(&format!("\"{}\"", pid))
        }

        // cmd 가 ping 자식을 낳는 2단계 트리. ping 30s 동안 생존(>NUL 로 출력 억제).
        let mut child = Command::new("cmd")
            .args(["/c", "ping -n 30 127.0.0.1 >NUL"])
            .spawn()
            .expect("spawn cmd");
        let pid = child.id();
        std::thread::sleep(Duration::from_millis(700)); // ping 자식 기동 대기

        jobkill::assign("test-slot-A", pid);
        assert!(alive(pid), "assign 직후 cmd({}) 가 이미 죽음 — 예상밖", pid);

        jobkill::close("test-slot-A"); // 핸들 닫힘 → KILL_ON_JOB_CLOSE 발동

        let start = Instant::now();
        let mut dead = false;
        while start.elapsed() < Duration::from_secs(5) {
            if !alive(pid) {
                dead = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        let _ = child.kill(); // 안전망(이미 종료됐어야 함)
        assert!(dead, "close() 후에도 cmd pid {} 생존 — 슬롯 job-close reap 미작동", pid);
    }
}
