// 봇→앱 동기 REST 인테이크 (ADR-001 / SLOTRUNNER-FRD-001 F001).
//
// - 127.0.0.1 만 바인드(외부 노출 금지, ADR-001 보안).
// - 자체 스레드 + 동기 처리(tiny_http). 비동기 런타임 미도입.
// - 라우팅 결정은 순수 함수 `decide`(테스트 가능). 부수효과(emit)·응답은 handle 에서.
// - 큐(대기열)는 프론트(Zustand)가 소유 → queue:clear 는 이벤트로 위임(프론트가 비우고 콘솔 보고).

use std::io::{Cursor, Read};

use tauri::{AppHandle, Emitter};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::domain::job::{Job, JobSpec};

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

fn parse_validate(body: &str) -> Result<JobSpec, String> {
    let spec: JobSpec =
        serde_json::from_str(body).map_err(|e| format!("파싱 실패: {}", e))?;
    spec.validate()?;
    Ok(spec)
}

fn decide(method: &Method, path: &str, body: &str) -> Decision {
    match (method, path) {
        (Method::Get, "/health") => Decision::Health,
        (Method::Post, "/jobs") => match parse_validate(body) {
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

    let resp = match decide(&method, &path, &body) {
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
            json(400, serde_json::json!({ "error": "JOB_SPEC_INVALID", "reason": reason }))
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

    const FULL: &str = r#"{"cwd":"C:/repo","app":"MASTER","phase":"p","sln":"s","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#;

    #[test]
    fn health_route() {
        assert_eq!(decide(&Method::Get, "/health", ""), Decision::Health);
    }

    #[test]
    fn jobs_valid_route() {
        match decide(&Method::Post, "/jobs", FULL) {
            Decision::NewJob(s) => assert_eq!(s.phase, "p"),
            other => panic!("expected NewJob, got {:?}", other),
        }
    }

    #[test]
    fn jobs_invalid_route() {
        // 필수 누락 → JobError
        match decide(&Method::Post, "/jobs", r#"{"phase":"p"}"#) {
            Decision::JobError(_) => {}
            other => panic!("expected JobError, got {:?}", other),
        }
        // 빈 필드 → JobError(validate)
        let empty = r#"{"cwd":"","app":"M","phase":"p","sln":"s","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#;
        match decide(&Method::Post, "/jobs", empty) {
            Decision::JobError(_) => {}
            other => panic!("expected JobError(validate), got {:?}", other),
        }
    }

    #[test]
    fn queue_clear_route() {
        assert_eq!(decide(&Method::Post, "/jobs/queue:clear", ""), Decision::QueueClear);
    }

    #[test]
    fn unknown_route() {
        assert_eq!(decide(&Method::Get, "/nope", ""), Decision::NotFound);
        assert_eq!(decide(&Method::Post, "/jobs/x", ""), Decision::NotFound);
    }
}
