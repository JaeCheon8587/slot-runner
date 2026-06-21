// 봇→앱 동기 REST 인테이크 (ADR-001 / SLOTRUNNER-FRD-001 F001).
//
// - 127.0.0.1 만 바인드(외부 노출 금지, ADR-001 보안).
// - 자체 스레드 + 동기 처리(tiny_http). 비동기 런타임 미도입.
// - 라우팅 결정은 순수 함수 `decide`(테스트 가능). 부수효과(emit)·응답은 handle 에서.
// - 큐(대기열)는 프론트(Zustand)가 소유 → queue:clear 는 이벤트로 위임(프론트가 비우고 콘솔 보고).

use std::io::{Cursor, Read};

use tauri::{AppHandle, Emitter};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::domain::job::{Job, JobSpec, Projects};

/// 본문 상한 — 과대 페이로드 방어.
const MAX_BODY: u64 = 256 * 1024;

/// 라우팅 결정(순수). emit/uuid 같은 부수효과는 포함하지 않는다 → 단위테스트 대상.
#[derive(Debug, PartialEq)]
enum Decision {
    Health,
    NewJob(JobSpec),
    JobError(String),
    QueueClear,
    NotFound,
}

/// 파싱 → project 해석(레지스트리) → 필수 검증. 어느 단계든 실패하면 사유 반환(JOB_SPEC_INVALID/PROJECT_UNKNOWN).
fn parse_resolve_validate(body: &str, registry: &Projects) -> Result<JobSpec, String> {
    let mut spec: JobSpec =
        serde_json::from_str(body).map_err(|e| format!("파싱 실패: {}", e))?;
    spec.resolve(registry)?;
    spec.validate()?;
    Ok(spec)
}

fn decide(method: &Method, path: &str, body: &str, registry: &Projects) -> Decision {
    match (method, path) {
        (Method::Get, "/health") => Decision::Health,
        (Method::Post, "/jobs") => match parse_resolve_validate(body, registry) {
            Ok(spec) => Decision::NewJob(spec),
            Err(e) => Decision::JobError(e),
        },
        (Method::Post, "/jobs/queue:clear") => Decision::QueueClear,
        _ => Decision::NotFound,
    }
}

/// REST 서버를 백그라운드 스레드에서 시작한다. 바인드 실패만 에러로 반환(호출부가 로깅).
pub fn start(app: AppHandle, port: u16) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", port);
    let server =
        Server::http(&addr).map_err(|e| format!("REST 바인드 실패 ({}): {}", addr, e))?;
    std::thread::Builder::new()
        .name("rest-server".into())
        .spawn(move || {
            eprintln!("[rest] listening on http://{}", addr);
            for req in server.incoming_requests() {
                handle(&app, req);
            }
        })
        .map_err(|e| format!("REST 스레드 생성 실패: {}", e))?;
    Ok(())
}

fn json(code: u16, body: serde_json::Value) -> Response<Cursor<Vec<u8>>> {
    let data = serde_json::to_vec(&body).unwrap_or_else(|_| b"{}".to_vec());
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("정적 헤더");
    Response::from_data(data).with_status_code(code).with_header(header)
}

fn handle(app: &AppHandle, mut req: Request) {
    let method = req.method().clone();
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or("").to_string();

    // 본문 읽기(상한). GET 등은 보통 빈 본문.
    let mut body = String::new();
    if req.as_reader().take(MAX_BODY).read_to_string(&mut body).is_err() {
        let _ = req.respond(json(400, serde_json::json!({
            "error": "JOB_SPEC_INVALID", "reason": "본문 읽기 실패(UTF-8 아님)"
        })));
        return;
    }

    // 프로젝트 레지스트리(projects.json) 로드 — project 논리명 해석용(ADR-009). 매 요청 로드(소형 파일).
    let registry = crate::config::load_projects(app);
    let resp = match decide(&method, &path, &body, &registry) {
        Decision::Health => json(200, serde_json::json!({ "status": "ok" })),
        Decision::NewJob(spec) => {
            let job_id = uuid::Uuid::new_v4().to_string();
            let job = Job { job_id: job_id.clone(), spec };
            if let Err(e) = app.emit("job:request", &job) {
                json(500, serde_json::json!({ "error": "EMIT_FAILED", "reason": e.to_string() }))
            } else {
                json(202, serde_json::json!({ "job_id": job_id, "status": "accepted" }))
            }
        }
        Decision::JobError(reason) => {
            // 미등록 프로젝트는 독립 errorCode 로 구분(PRD 부록 B / ADR-009) — 봇이 구분 처리.
            let code = if reason.starts_with("PROJECT_UNKNOWN") {
                "PROJECT_UNKNOWN"
            } else {
                "JOB_SPEC_INVALID"
            };
            json(400, serde_json::json!({ "error": code, "reason": reason }))
        }
        Decision::QueueClear => {
            // 큐는 프론트 소유 → 비우기 이벤트 위임(프론트가 실제 비우고 콘솔 보고).
            let _ = app.emit("queue:clear", ());
            json(200, serde_json::json!({ "status": "clear_requested" }))
        }
        Decision::NotFound => json(404, serde_json::json!({ "error": "NOT_FOUND" })),
    };
    let _ = req.respond(resp);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::job::ProjectEntry;

    const FULL: &str = r#"{"cwd":"C:/repo","app":"MASTER","phase":"p","sln":"s","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#;

    fn empty_reg() -> Projects {
        Projects::new()
    }
    fn xlab_reg() -> Projects {
        let mut m = Projects::new();
        m.insert(
            "xlab".into(),
            ProjectEntry {
                cwd: "C:/XLab".into(),
                sln: "X.sln".into(),
                app: "MASTER".into(),
                test_target: None,
            },
        );
        m
    }

    #[test]
    fn health_route() {
        assert_eq!(decide(&Method::Get, "/health", "", &empty_reg()), Decision::Health);
    }

    #[test]
    fn jobs_valid_route() {
        // 직접 지정(A 폴백) — 레지스트리 비어도 통과.
        match decide(&Method::Post, "/jobs", FULL, &empty_reg()) {
            Decision::NewJob(s) => assert_eq!(s.phase, "p"),
            other => panic!("expected NewJob, got {:?}", other),
        }
    }

    #[test]
    fn jobs_project_resolved_route() {
        // project 만 보내면 레지스트리로 cwd/sln/app 해석 후 통과(B 방식).
        let body = r#"{"project":"xlab","phase":"p","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#;
        match decide(&Method::Post, "/jobs", body, &xlab_reg()) {
            Decision::NewJob(s) => {
                assert_eq!(s.cwd, "C:/XLab");
                assert_eq!(s.sln, "X.sln");
                assert_eq!(s.app, "MASTER");
            }
            other => panic!("expected NewJob, got {:?}", other),
        }
    }

    #[test]
    fn jobs_unknown_project_route() {
        let body = r#"{"project":"nope","phase":"p","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#;
        match decide(&Method::Post, "/jobs", body, &xlab_reg()) {
            Decision::JobError(e) => assert!(e.contains("PROJECT_UNKNOWN")),
            other => panic!("expected JobError(PROJECT_UNKNOWN), got {:?}", other),
        }
    }

    #[test]
    fn jobs_invalid_route() {
        // 필수 누락(board_id 등) → 역직렬화 실패 → JobError
        match decide(&Method::Post, "/jobs", r#"{"phase":"p"}"#, &empty_reg()) {
            Decision::JobError(_) => {}
            other => panic!("expected JobError, got {:?}", other),
        }
        // project 없고 cwd 도 비면 → validate 실패(해석도 직접지정도 없음)
        let empty = r#"{"cwd":"","app":"","phase":"p","sln":"","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#;
        match decide(&Method::Post, "/jobs", empty, &empty_reg()) {
            Decision::JobError(_) => {}
            other => panic!("expected JobError(validate), got {:?}", other),
        }
    }

    #[test]
    fn queue_clear_route() {
        assert_eq!(
            decide(&Method::Post, "/jobs/queue:clear", "", &empty_reg()),
            Decision::QueueClear
        );
    }

    #[test]
    fn unknown_route() {
        assert_eq!(decide(&Method::Get, "/nope", "", &empty_reg()), Decision::NotFound);
        assert_eq!(decide(&Method::Post, "/jobs/x", "", &empty_reg()), Decision::NotFound);
    }
}
