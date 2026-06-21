use serde::{Deserialize, Serialize};

/// 봇이 보내는 잡 명세 (REST 본문). 필수 항목 누락 시 역직렬화 실패 → JOB_SPEC_INVALID.
/// SLOTRUNNER-FRD-001 §9 입출력 개념 대응. 코드상세는 본 타입이 SSOT(FRD 는 개념만).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    /// 대상 repo 경로 (슬롯 claude 세션의 cwd). 루틴(docs-add-task→forge-scope→ddr-loop)이 이 repo 에서 동작.
    pub cwd: String,
    /// 대상 App 코드 (docs-add-task 입력, 예: MASTER). v0.7 per-App SSOT.
    pub app: String,
    /// 작업 식별 슬러그. forge 워크트리 격리 키(ADR-002 — 잡 단위 격리).
    pub phase: String,
    /// 빌드 대상 솔루션 (forge contract-TDD).
    pub sln: String,
    /// 빌드 스코프 축소용 test csproj (선택).
    #[serde(default)]
    pub test_target: Option<String>,
    /// 자연어 요구사항 (docs-add-task 가 설계문서로 변환). 루틴의 핵심 입력.
    #[serde(default)]
    pub prompt: String,
    /// Monday 통지 대상.
    pub board_id: String,
    pub item_id: String,
    pub update_id: String,
}

impl JobSpec {
    /// 필수 문자열 필드가 비어있지 않은지 검증. 비면 누락 항목명 반환.
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
}
