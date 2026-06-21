use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 프로젝트 레지스트리 항목 — 호스트 로컬 경로·빌드 파라미터(프로젝트당 고정).
/// SlotRunner 호스트가 소유(projects.json). 봇은 논리명(project)만 보낸다 — ADR-009.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectEntry {
    /// 대상 repo 절대경로 (슬롯 claude 세션의 cwd).
    pub cwd: String,
    /// 빌드 대상 솔루션 (forge contract-TDD).
    pub sln: String,
    /// 대상 App 코드 (docs-add-task 입력, 예: MASTER).
    pub app: String,
    /// 빌드 스코프 축소용 test csproj (선택).
    #[serde(default)]
    pub test_target: Option<String>,
}

/// 프로젝트 논리명 → 항목. projects.json 의 역직렬화 대상.
pub type Projects = HashMap<String, ProjectEntry>;

/// 봇이 보내는 잡 명세 (REST 본문). cwd/sln/app 은 project 로 해석되거나 직접 지정.
/// SLOTRUNNER-FRD-001 §9 입출력 개념 대응. 코드상세는 본 타입이 SSOT(FRD 는 개념만).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    /// 프로젝트 논리명(예: "xlab"·"smartros"). 지정 시 레지스트리에서 cwd/sln/app/test_target 해석 — ADR-009.
    #[serde(default)]
    pub project: String,
    /// 대상 repo 경로 (슬롯 claude 세션의 cwd). project 로 해석되거나 직접 지정(직접값 우선).
    #[serde(default)]
    pub cwd: String,
    /// 대상 App 코드 (docs-add-task 입력, 예: MASTER). v0.7 per-App SSOT. project 로 해석 가능.
    #[serde(default)]
    pub app: String,
    /// 작업 식별 슬러그. forge 워크트리 격리 키(ADR-002 — 잡 단위 격리).
    pub phase: String,
    /// 빌드 대상 솔루션 (forge contract-TDD). project 로 해석 가능.
    #[serde(default)]
    pub sln: String,
    /// 빌드 스코프 축소용 test csproj (선택). project 로 해석 가능.
    #[serde(default)]
    pub test_target: Option<String>,
    /// 자연어 요구사항 (docs-add-task 가 설계문서로 변환). 루틴의 핵심 입력.
    #[serde(default)]
    pub prompt: String,
    /// 입력 문서 — A(설계)=요구사항 .md / B(개발)=TASK .md. 비면 prompt 만 사용.
    #[serde(default)]
    pub doc: String,
    /// 잡별 routine(스텝 순서). 비면 프론트가 기본 풀 routine 적용. 봇이 Monday 키워드로 채움.
    #[serde(default)]
    pub stages: Vec<String>,
    /// Monday 통지 대상.
    pub board_id: String,
    pub item_id: String,
    pub update_id: String,
}

impl JobSpec {
    /// project 논리명을 레지스트리로 해석해 비어있는 cwd/sln/app/test_target 을 채운다(직접 지정값 우선).
    /// project 가 비면 직접 지정 모드(no-op). project 가 있는데 레지스트리에 없으면 PROJECT_UNKNOWN.
    pub fn resolve(&mut self, registry: &Projects) -> Result<(), String> {
        let name = self.project.trim();
        if name.is_empty() {
            return Ok(()); // 직접 지정(cwd/sln/app) 모드
        }
        let e = registry
            .get(name)
            .ok_or_else(|| format!("PROJECT_UNKNOWN: 알 수 없는 프로젝트 '{}'", name))?;
        if self.cwd.trim().is_empty() {
            self.cwd = e.cwd.clone();
        }
        if self.sln.trim().is_empty() {
            self.sln = e.sln.clone();
        }
        if self.app.trim().is_empty() {
            self.app = e.app.clone();
        }
        if self.test_target.is_none() {
            self.test_target = e.test_target.clone();
        }
        Ok(())
    }

    /// 필수 문자열 필드가 비어있지 않은지 검증(resolve 이후 호출). 비면 누락 항목명 반환.
    pub fn validate(&self) -> Result<(), String> {
        let required = [
            ("cwd", &self.cwd),
            ("app", &self.app),
            ("phase", &self.phase),
            ("sln", &self.sln),
            ("prompt", &self.prompt),
            ("board_id", &self.board_id),
            ("item_id", &self.item_id),
            ("update_id", &self.update_id),
        ];
        for (name, val) in required {
            if val.trim().is_empty() {
                return Err(format!("{} 비어있음", name));
            }
        }
        Ok(())
    }
}

/// 접수되어 job_id 가 부여된 잡. 프론트로 emit 되는 페이로드.
#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub job_id: String,
    #[serde(flatten)]
    pub spec: JobSpec,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_json() -> &'static str {
        r#"{"cwd":"C:/repo","app":"MASTER","phase":"loader-task-001","sln":"Src/X.sln",
            "board_id":"1","item_id":"2","update_id":"3","prompt":"두 정수 합 엔드포인트"}"#
    }

    #[test]
    fn full_spec_parses_and_validates() {
        let s: JobSpec = serde_json::from_str(full_json()).unwrap();
        assert!(s.validate().is_ok());
        assert_eq!(s.phase, "loader-task-001");
        assert_eq!(s.test_target, None);
    }

    #[test]
    fn missing_required_field_fails_deserialization() {
        // board_id 등 필수 누락 → serde 역직렬화 실패(JOB_SPEC_INVALID 경로).
        assert!(serde_json::from_str::<JobSpec>(r#"{"phase":"p","doc":"d","sln":"s"}"#).is_err());
    }

    #[test]
    fn empty_required_field_fails_validate() {
        let s: JobSpec = serde_json::from_str(
            r#"{"cwd":"C:/repo","app":"MASTER","phase":"  ","sln":"s","prompt":"p","board_id":"1","item_id":"2","update_id":"3"}"#,
        )
        .unwrap();
        assert!(s.validate().is_err());
    }

    fn registry() -> Projects {
        let mut m = Projects::new();
        m.insert(
            "xlab".into(),
            ProjectEntry {
                cwd: "C:/XLab".into(),
                sln: "X.sln".into(),
                app: "MASTER".into(),
                test_target: Some("X.Test.csproj".into()),
            },
        );
        m
    }

    #[test]
    fn resolve_fills_from_registry_then_validates() {
        // project 만 + cwd/sln/app 비움 → 레지스트리로 채워지고 validate 통과.
        let mut s: JobSpec = serde_json::from_str(
            r#"{"project":"xlab","phase":"p","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#,
        )
        .unwrap();
        assert!(s.cwd.is_empty());
        s.resolve(&registry()).unwrap();
        assert_eq!(s.cwd, "C:/XLab");
        assert_eq!(s.sln, "X.sln");
        assert_eq!(s.app, "MASTER");
        assert_eq!(s.test_target.as_deref(), Some("X.Test.csproj"));
        assert!(s.validate().is_ok());
    }

    #[test]
    fn resolve_unknown_project_errors() {
        let mut s: JobSpec = serde_json::from_str(
            r#"{"project":"nope","phase":"p","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#,
        )
        .unwrap();
        let e = s.resolve(&registry()).unwrap_err();
        assert!(e.contains("PROJECT_UNKNOWN"));
    }

    #[test]
    fn explicit_cwd_overrides_and_no_project_is_noop() {
        // project 없이 직접 지정(A 폴백) → resolve no-op, validate 통과.
        let mut s: JobSpec = serde_json::from_str(
            r#"{"cwd":"C:/repo","app":"M","sln":"s","phase":"p","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#,
        )
        .unwrap();
        s.resolve(&registry()).unwrap();
        assert_eq!(s.cwd, "C:/repo");
        assert!(s.validate().is_ok());
        // project + 직접 cwd 동시 → 직접값 우선(덮어쓰지 않음).
        let mut s2: JobSpec = serde_json::from_str(
            r#"{"project":"xlab","cwd":"C:/override","phase":"p","prompt":"x","board_id":"1","item_id":"2","update_id":"3"}"#,
        )
        .unwrap();
        s2.resolve(&registry()).unwrap();
        assert_eq!(s2.cwd, "C:/override"); // 직접값 우선
        assert_eq!(s2.sln, "X.sln"); // 비었던 건 레지스트리로
    }
}
