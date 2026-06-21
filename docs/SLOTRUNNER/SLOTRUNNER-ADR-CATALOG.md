# SLOTRUNNER-ADR-CATALOG — SlotRunner ADR Catalog

> ADR 결정 인덱스. 단일 App(`SLOTRUNNER`). 새 ADR 등재 시 [ADR 폴더](ADR/) 의 개별 파일(`SLOTRUNNER-ADR-{NNN}.md`) 신규 + 본 카탈로그 행 추가(2곳 동기화).
> 식별자 규약은 [DOCUMENT_GUIDE §5](../DOCUMENT_GUIDE.md#5-식별자-규약).

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-CATALOG |
| 작성 가정 | ADR 본문(개별 파일)과 1:1 동기화. 본 카탈로그가 상태/영향/반영 SSOT |
| 관련 문서 | [ADR 폴더](ADR/) · [SLOTRUNNER-PRD](SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](SLOTRUNNER-ARCHITECTURE.md) · [솔루션 ARCHITECTURE](../ARCHITECTURE.md) · [/CLAUDE.md](../../CLAUDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 — 부트스트랩(ADR 본문 미등재). 설계 단계 결정은 후보로 기재 | jaecheon.jeong |
| 0.2 | 2026-06-21 | ADR-006(슬롯별 Job Object 고아 방지)·ADR-007(컨텍스트 임계 /compact) 등재 | jaecheon.jeong |
| 0.3 | 2026-06-21 | ADR-008(토스트 통지·자동복구 배제) 등재 | jaecheon.jeong |
| 0.4 | 2026-06-21 | ADR-009(프로젝트 레지스트리·봇 논리명 책임분리) 등재 | jaecheon.jeong |
| 0.5 | 2026-06-21 | ADR-010(봇 통합·루틴 프리셋 책임 경계) 등재 | jaecheon.jeong |

---

## Accepted

> 채택된 결정. 영향 범위·반영 문서는 본 표가 SSOT.

| ADR | 제목 | 일자 | 영향 범위 | 영향 모듈 | 반영 문서 |
|---|---|---|---|---|---|
| [SLOTRUNNER-ADR-001](ADR/SLOTRUNNER-ADR-001.md) | 봇↔앱 통신은 동기 REST 인테이크(tiny_http) | 2026-06-20 | 봇 결합·수신 경계 | Infrastructure(REST), Application(JobIntake) | [PRD §8](SLOTRUNNER-PRD.md#8-비기능-요구사항-app-특화) · [FC F001](SLOTRUNNER-FC.md) · [FRD-001](FRD/SLOTRUNNER-FRD-001.md) |
| [SLOTRUNNER-ADR-002](ADR/SLOTRUNNER-ADR-002.md) | 처리 모델은 N슬롯 풀 + FIFO 큐(세션 격리, 유지=점유) | 2026-06-20 | 동시성·슬롯 생명주기 | Application(SlotPool/Queue), Presentation(슬롯 그리드) | [PRD §6·§9](SLOTRUNNER-PRD.md#6-핵심-시나리오-본-app-내부) · [FC F002](SLOTRUNNER-FC.md) · [FRD-002](FRD/SLOTRUNNER-FRD-002.md) |
| [SLOTRUNNER-ADR-003](ADR/SLOTRUNNER-ADR-003.md) | 단계 게이트는 파일 판정, 종료/유지는 사람 결정 모달 | 2026-06-20 | 단계 전이·마감 | Application(Pipeline), Infrastructure(Gate), Presentation(EndOfRunModal) | [PRD §6·§8](SLOTRUNNER-PRD.md#6-핵심-시나리오-본-app-내부) · [FC F003·F004](SLOTRUNNER-FC.md) · [FRD-001](FRD/SLOTRUNNER-FRD-001.md) |
| [SLOTRUNNER-ADR-004](ADR/SLOTRUNNER-ADR-004.md) | 운영 상태는 휘발, 단계는 7200s 타임아웃으로 보호 | 2026-06-20 | 상태 영속·단계 수명 | Application(Pipeline/SlotPool), Infrastructure(PTY) | [PRD §8·§9](SLOTRUNNER-PRD.md#8-비기능-요구사항-app-특화) · [FRD-001 §7·§13](FRD/SLOTRUNNER-FRD-001.md) · [FRD-002 §10](FRD/SLOTRUNNER-FRD-002.md) |
| [SLOTRUNNER-ADR-005](ADR/SLOTRUNNER-ADR-005.md) | 취소·개입은 슬롯 PTY 직접 입력, REST 취소 없음 | 2026-06-20 | 취소·수동 개입·슬롯 UI | Presentation(슬롯 포커스/입력), Application(SlotPool) | [PRD §3.1·§6 S8](SLOTRUNNER-PRD.md#31-릴리즈-범위-본-app-한정) · [FC F006](SLOTRUNNER-FC.md) · [FRD-002 §5·§11](FRD/SLOTRUNNER-FRD-002.md) |
| [SLOTRUNNER-ADR-006](ADR/SLOTRUNNER-ADR-006.md) | 슬롯 자식 트리는 슬롯별 Job Object(KILL_ON_JOB_CLOSE)로 정리(고아 방지) | 2026-06-21 | 슬롯 생명주기·프로세스 정리 | Infrastructure(PTY/JobObject) | [PRD §8](SLOTRUNNER-PRD.md#8-비기능-요구사항-app-특화) · [FRD-002 §10](FRD/SLOTRUNNER-FRD-002.md) |
| [SLOTRUNNER-ADR-007](ADR/SLOTRUNNER-ADR-007.md) | 스텝 전이 시 컨텍스트 점유 임계(기본 40%) 기반 /compact 자동 주입 | 2026-06-21 | 컨텍스트 관리·단계 전이 | Presentation(StageController), Infrastructure(ContextUsage/PTY) | [PRD §3.1·§7·§8](SLOTRUNNER-PRD.md#31-릴리즈-범위-본-app-한정) · [FC F007](SLOTRUNNER-FC.md) · [FRD-003](FRD/SLOTRUNNER-FRD-003.md) |
| [SLOTRUNNER-ADR-008](ADR/SLOTRUNNER-ADR-008.md) | 스톨·완료는 데스크톱 토스트로 통지, 자동 복구(넛지) 없음(선택권 보존) | 2026-06-21 | 통지·스톨 대응·운영자 개입 | Presentation(NotificationBridge), Infrastructure(notification plugin) | [PRD §3.1·§7·§8](SLOTRUNNER-PRD.md#31-릴리즈-범위-본-app-한정) · [FC F008](SLOTRUNNER-FC.md) · [FRD-004](FRD/SLOTRUNNER-FRD-004.md) |
| [SLOTRUNNER-ADR-009](ADR/SLOTRUNNER-ADR-009.md) | 대상 경로·빌드 파라미터는 SlotRunner 프로젝트 레지스트리가 소유, 봇은 논리명(project)만 | 2026-06-21 | 봇 결합·인테이크·다중 프로젝트 | Domain(JobSpec/resolve), Infrastructure(config/REST) | [PRD §5·부록 B](SLOTRUNNER-PRD.md) · [FRD-001 §8·§9](FRD/SLOTRUNNER-FRD-001.md) |
| [SLOTRUNNER-ADR-010](ADR/SLOTRUNNER-ADR-010.md) | 봇 통합 — 루틴(stages) 매핑은 봇, SlotRunner는 stages 실행 + Monday 통지 | 2026-06-21 | 봇↔앱 책임 경계·루틴 결정 | Infrastructure(REST), 외부(agentorchestrator) | [PRD §5·§6](SLOTRUNNER-PRD.md) · [FRD-001 §5·§8·§9](FRD/SLOTRUNNER-FRD-001.md) |

## Proposed

> 제안 중. 본 부트스트랩 시점엔 정식 ADR 번호 미부여. ADR 본문 작성 시 NNN 부여 후 행 이전.

| ADR | 제목 | 제안 일자 | 영향 범위 | 결정 기한 | 결정 필요자 |
|---|---|---|---|---|---|
| 없음 | 없음 | 없음 | 없음 | 없음 | 없음 |

## Deprecated / Superseded
| ADR | 제목 | Deprecated 일자 | 후속 ADR | 사유 |
|---|---|---|---|---|
| 없음 | 없음 | 없음 | 없음 | 없음 |

---

## 결정 후보 → ADR 매핑 (본문화 완료)

> 설계 대화의 결정 후보 C1~C6 은 아래와 같이 정식 ADR 로 본문화됨(위 Accepted 표).

| 후보 | 본문화 ADR |
|---|---|
| C1 (REST, WS 배제) · C2 (tiny_http, axum/tokio 배제) | [SLOTRUNNER-ADR-001](ADR/SLOTRUNNER-ADR-001.md) |
| C3 (N슬롯 풀+FIFO 큐) · C5 (유지=점유) | [SLOTRUNNER-ADR-002](ADR/SLOTRUNNER-ADR-002.md) |
| C4 (EndOfRunModal 결정 게이트) · C6 (파일 단계 게이트) | [SLOTRUNNER-ADR-003](ADR/SLOTRUNNER-ADR-003.md) |
| 보강(휘발·타임아웃) | [SLOTRUNNER-ADR-004](ADR/SLOTRUNNER-ADR-004.md) |
| 보강(수동 취소=PTY 직접 입력) | [SLOTRUNNER-ADR-005](ADR/SLOTRUNNER-ADR-005.md) |
| 구현 후 추가(고아 방지=슬롯별 Job Object) | [SLOTRUNNER-ADR-006](ADR/SLOTRUNNER-ADR-006.md) |
| 구현 후 추가(컨텍스트 임계 /compact) | [SLOTRUNNER-ADR-007](ADR/SLOTRUNNER-ADR-007.md) |
| 구현 후 추가(토스트 통지·자동복구 배제) | [SLOTRUNNER-ADR-008](ADR/SLOTRUNNER-ADR-008.md) |
| 구현 후 추가(프로젝트 레지스트리·책임분리) | [SLOTRUNNER-ADR-009](ADR/SLOTRUNNER-ADR-009.md) |
| 구현 후 추가(봇 통합·루틴 프리셋) | [SLOTRUNNER-ADR-010](ADR/SLOTRUNNER-ADR-010.md) |
