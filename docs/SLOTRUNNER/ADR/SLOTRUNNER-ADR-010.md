# SLOTRUNNER-ADR-010 — 봇 통합: 메시지→루틴 매핑은 봇, SlotRunner는 stages 실행 + Monday 통지

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-010 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | Slack 봇(agentorchestrator, 외부 자산)이 Monday pulse 링크 메시지를 받아 SlotRunner REST 로 위임. 봇은 로컬 파이프라인을 직접 돌리지 않고 POST 만 한다 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ADR-001](SLOTRUNNER-ADR-001.md) · [SLOTRUNNER-ADR-009](SLOTRUNNER-ADR-009.md) · [FRD-001](../FRD/SLOTRUNNER-FRD-001.md) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-21 | 초안 — 봇 통합 계약·루틴 프리셋 본문화 | jaecheon.jeong |

---

## ADR-010: 루틴 결정(설계/개발)은 봇이 메시지 신호로 규칙 매핑하고, SlotRunner 는 받은 stages 를 그대로 실행하며 Monday 통지까지 종결한다

- **상태**: Accepted (2026-06-21)
- **우선순위**: P1
- **컨텍스트**:
  - 봇은 Slack 메시지(Monday pulse 링크 + 입력 문서 경로 + 진행 키워드)를 받는다.
  - 입력 문서 종류·키워드가 곧 작업 종류다: `.requirements/…md`(요구사항) = 설계, `…/TASK/…md`(TASK) = 개발.
  - 기존 봇은 로컬에서 `claude -p` 헤드리스 파이프라인을 직접 돌렸다(워커풀·subprocess) → SlotRunner 슬롯풀·영속세션·고아정리와 중복.
- **결정**:
  - **루틴 결정 = 봇 책임(규칙 기반, LLM 불필요)**: 입력 문서/키워드 → `stages` 배열로 매핑.
    - 설계(`.requirements/…md` 또는 "설계") → `["docs-add-task","forge-scope","ddr-loop"]`
    - 개발(`…/TASK/…md` 또는 "개발") → `["forge-scope","ddr-loop"]`
  - **SlotRunner = stages 그대로 실행**: 루틴 해석을 SlotRunner 가 다시 하지 않는다. 받은 stages 순서대로 스텝 루프 구동.
  - **위임 경계**: 봇은 파싱 → 루틴·프로젝트 결정 → `POST /jobs` 까지. 파이프라인 실행·Monday 통지는 SlotRunner(슬롯 세션). 봇은 Slack 에 접수 응답만.
  - **POST 바디**: `{ project, phase, stages, prompt, doc?, board_id, item_id, update_id }` — 경로는 project 로 해석([ADR-009]).
  - **프로젝트**: board_id → project 논리명(xlab·smartros)은 봇이 매핑. 경로/빌드값은 SlotRunner 레지스트리 소유([ADR-009]).
- **결과**:
  - 가능: 중복 제거(봇 얇아짐), 영속 세션 재사용, 단일 슬롯풀·고아정리·컨텍스트압축·토스트 일원화.
  - 제약: 봇 취소는 SlotRunner 에 프로그램적 per-job 취소가 없어([ADR-005]) 큐 비우기(queue:clear)로만 위임 — per-job 취소는 후속 과제.
- **대안 검토**:
  - 옵션 (SlotRunner 가 메시지 원문 받아 루틴 판정): 기각 — SlotRunner 는 Slack/Monday 파싱을 모름(봇 책임 경계 유지).
  - 옵션 (봇이 계속 로컬 파이프라인): 기각 — 중복·관리포인트 분산(프로젝트 본래 목표 위배).

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §5 이해관계자·§6 시나리오(봇 위임)
- [SLOTRUNNER-FRD-001](../FRD/SLOTRUNNER-FRD-001.md) — §5 흐름·§8 루틴 프리셋·§9 입출력
