# SLOTRUNNER-ADR-009 — 대상 경로·빌드 파라미터는 SlotRunner 프로젝트 레지스트리가 소유, 봇은 논리명만 보낸다

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-009 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | SlotRunner 를 여러 프로젝트(예: XLab·SmartROS)에 사용. 대상 repo 경로는 SlotRunner 호스트의 로컬 경로(그 PC에서만 유효)이며 프로젝트당 고정 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ADR-001](SLOTRUNNER-ADR-001.md) · [FRD-001](../FRD/SLOTRUNNER-FRD-001.md) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-21 | 초안 — 프로젝트 레지스트리·책임 분리 결정 본문화 | jaecheon.jeong |

---

## ADR-009: 봇은 프로젝트 논리명(project)만 보내고, SlotRunner 호스트의 프로젝트 레지스트리(projects.json)가 cwd/sln/app/test_target 을 해석한다

- **상태**: Accepted (2026-06-21)
- **우선순위**: P1
- **컨텍스트**:
  - SlotRunner 를 여러 repo(XLab·SmartROS …)에 쓴다. 잡마다 대상 경로가 다르다.
  - 대상 repo 경로는 **SlotRunner 가 도는 호스트의 로컬 경로** — 그 PC에서만 의미 있다. `sln`·`test_target`·`app` 도 **프로젝트당 고정**(잡마다 안 바뀜).
  - 봇이 POST 에 풀 경로(cwd)를 박으면, 봇이 **남의 PC 파일시스템 레이아웃**을 알아야 하고 경로 변경 시 봇을 고쳐야 한다(결합).
- **결정**:
  - **레지스트리 소유 = SlotRunner 호스트**: `app_config_dir/projects.json` 에 `논리명 → { cwd, sln, app, test_target? }` 등록(호스트 로컬, 레포 미커밋).
  - **봇은 논리명만**: POST 에 `project: "xlab"` + phase·prompt·stages·doc?·Monday ID. SlotRunner 가 레지스트리로 cwd/sln/app/test_target 을 **해석(resolve)** 후 슬롯 구동.
  - **직접 지정 폴백(A)**: `project` 없이 cwd/sln/app 을 직접 보내면 그대로 사용(임시·테스트). 직접값은 레지스트리보다 우선(override).
  - **알 수 없는 project → 거부**(`PROJECT_UNKNOWN`, 4xx).
  - 해석은 인테이크(REST) 단계에서 수행 — 프론트로는 **해석 완료된 Job** 이 전달(프론트 무변경).
- **결과**:
  - 가능: 봇 디커플(파일경로 모름, 논리명만), 페이로드 축소(프로젝트당 고정값 1곳 정의), 경로 변경은 호스트 레지스트리만 수정.
  - 제약: 호스트에 projects.json 선등록 필요(미등록 프로젝트는 PROJECT_UNKNOWN). 레지스트리는 호스트 로컬이라 레포에 없음(example 만 커밋).
- **대안 검토**:
  - 옵션 A (봇이 cwd 풀경로 POST): 기각(주) — 봇이 호스트 경로 결합. 단 테스트·임시용 폴백으로 유지.
  - 옵션 (SlotRunner UI 설정 모달로 관리): 보류 — 현재는 projects.json 직접 편집. 후속 설정 모달과 통합 가능.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §5 이해관계자·부록 B errorCode(PROJECT_UNKNOWN)
- [SLOTRUNNER-FRD-001](../FRD/SLOTRUNNER-FRD-001.md) — §8 상세 요구·§9 입출력·§7 예외(E8)
