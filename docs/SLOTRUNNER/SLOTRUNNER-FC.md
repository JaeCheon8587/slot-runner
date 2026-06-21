# SLOTRUNNER-FC — SlotRunner Feature Catalog

> 본 FC 는 단일 App(`SLOTRUNNER`)의 기능 레지스트리. SYSTEM_CODE SSOT 는 [`/CLAUDE.md` Backend Services Overview](../../CLAUDE.md).
> 식별자 규약은 [DOCUMENT_GUIDE §5](../DOCUMENT_GUIDE.md#5-식별자-규약).

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-FC |
| 버전 | 0.1 (Draft) |
| 작성 가정 | 부트스트랩 — 핵심 기능 5종 등재. FRD 본문은 docs-add-task 단계에서 작성 |
| 관련 문서 | [SLOTRUNNER-PRD](SLOTRUNNER-PRD.md) · [SLOTRUNNER-ARCHITECTURE](SLOTRUNNER-ARCHITECTURE.md) · [SLOTRUNNER-ADR-CATALOG](SLOTRUNNER-ADR-CATALOG.md) · [FRD 폴더](FRD/) · [/CLAUDE.md](../../CLAUDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 — F001~F005 등재 | jaecheon.jeong |

> **기능 ID 규약**: `F{NNN}` 은 본 App 내 unique. 정식 F001~F099, Backlog F101~.

## App 개요
| 항목 | 요약 |
|---|---|
| App명 | SlotRunner |
| 역할 | 봇 REST 요청을 N슬롯 영속 Claude Code 세션으로 받아 파이프라인 구동·통지 |
| 목적 | 파이썬 래퍼 제거 + 세션 재사용 + 사람 결정 게이트(팝업) |
| 주요 기능 범위 | REST 인테이크 / 슬롯풀·큐 / 파이프라인 주입·게이트 / 종료·유지 팝업 / 공용 콘솔 |
| 범위 밖 | 봇·플러그인 자체 개발, 헤드리스 운영, 자격증명 저장 |

## 기능 레지스트리

### 기본 식별·설명
| 기능 ID | 기능명 | 기능 설명 | 기능 상태 | 구현 상태 | 테스트 상태 | 우선순위 |
|---|---|---|---|---|---|---|
| F001 | REST 잡 인테이크 | POST /jobs 접수→검증→큐 적재→202. 127.0.0.1 바인드 | Draft | Not Started | 미작성 | P0 |
| F002 | 슬롯 풀 + 큐 | 동시 N(기본 2) 슬롯 배정 + FIFO 큐(상한 10·초과 거부+Monday 통지·전체 비우기) + dequeue | Draft | Not Started | 미작성 | P1 |
| F003 | 파이프라인 주입·게이트 | 영속 세션에 forge-scope→ddr-loop→Monday 순차 주입, index.json/.review 결정적 게이트, 단계 타임아웃 7200s | Draft | Not Started | 미작성 | P0 |
| F004 | 종료/유지 팝업 | 완료/실패 EndOfRunModal + 슬롯 종료(unmount)/유지(점유) | Draft | Not Started | 미작성 | P0 |
| F005 | 공용 Hook 콘솔 | 전 슬롯 이벤트 panel_id 태그 머지 표시(최우측) | Draft | Not Started | 미작성 | P1 |
| F006 | 슬롯 PTY 직접 입력 | 슬롯 선택(포커스) + 키 입력으로 수동 개입·취소(프로그램적 취소 없음) | Draft | Not Started | 미작성 | P1 |

> **우선순위**: P0 = MVP 필수. P1 = 이번 릴리즈 권장. P2 = Backlog.

### 문서 연결
| 기능 ID | 관련 App PRD | 관련 FRD | 관련 API Spec | 관련 UI Spec | 관련 Data Spec |
|---|---|---|---|---|---|
| F001 | [PRD §7](SLOTRUNNER-PRD.md#7-주요-기능-요약-본-app-한정) | [SLOTRUNNER-FRD-001](FRD/SLOTRUNNER-FRD-001.md) | FRD-001 §9 인라인 | 미작성/추후 | 미작성/추후 |
| F002 | [PRD §3.1](SLOTRUNNER-PRD.md#31-릴리즈-범위-본-app-한정) | [SLOTRUNNER-FRD-002](FRD/SLOTRUNNER-FRD-002.md) | 미작성/추후 | FRD-002 §15 | 미작성/추후 |
| F003 | [PRD §7](SLOTRUNNER-PRD.md#7-주요-기능-요약-본-app-한정) | [SLOTRUNNER-FRD-001](FRD/SLOTRUNNER-FRD-001.md) | 미작성/추후 | 미작성/추후 | 미작성/추후 |
| F004 | [PRD §6](SLOTRUNNER-PRD.md#6-핵심-시나리오-본-app-내부) | [SLOTRUNNER-FRD-001](FRD/SLOTRUNNER-FRD-001.md) | 미작성/추후 | FRD-001 §15 | 미작성/추후 |
| F005 | [PRD §3.1](SLOTRUNNER-PRD.md#31-릴리즈-범위-본-app-한정) | [SLOTRUNNER-FRD-002](FRD/SLOTRUNNER-FRD-002.md) | 미작성/추후 | FRD-002 §15 | 미작성/추후 |
| F006 | [PRD §6](SLOTRUNNER-PRD.md#6-핵심-시나리오-본-app-내부) | [SLOTRUNNER-FRD-002](FRD/SLOTRUNNER-FRD-002.md) | 미작성/추후 | FRD-002 §15 | 미작성/추후 |

### 검증·근거·확인
| 기능 ID | 관련 Test Case | 수용 기준 | 요구 근거 | 확인 필요 여부 |
|---|---|---|---|---|
| F001 | [FRD-001 §18](FRD/SLOTRUNNER-FRD-001.md#18-테스트-관점) | [FRD-001 §17](FRD/SLOTRUNNER-FRD-001.md#17-수용-기준) | 요구사항 Q1(REST 채널) | 없음 |
| F002 | [FRD-002 §18](FRD/SLOTRUNNER-FRD-002.md#18-테스트-관점) | [FRD-002 §17](FRD/SLOTRUNNER-FRD-002.md#17-수용-기준) | 최종 그림(슬롯풀 N=2 + 큐) | 없음 |
| F003 | [FRD-001 §18](FRD/SLOTRUNNER-FRD-001.md#18-테스트-관점) | [FRD-001 §17](FRD/SLOTRUNNER-FRD-001.md#17-수용-기준) | 요구사항 Q2·Q3·Q5 | PTY 인터랙티브서 forge/ddr 동작 실측 필요 |
| F004 | [FRD-001 §18](FRD/SLOTRUNNER-FRD-001.md#18-테스트-관점) | [FRD-001 §17](FRD/SLOTRUNNER-FRD-001.md#17-수용-기준) | 요구사항 Q4 | 없음 |
| F005 | [FRD-002 §18](FRD/SLOTRUNNER-FRD-002.md#18-테스트-관점) | [FRD-002 §17](FRD/SLOTRUNNER-FRD-002.md#17-수용-기준) | 최종 그림(공용 콘솔 최우측) | 없음 |
| F006 | [FRD-002 §18](FRD/SLOTRUNNER-FRD-002.md#18-테스트-관점) | [FRD-002 §17](FRD/SLOTRUNNER-FRD-002.md#17-수용-기준) | 사용자 결정(수동 취소=PTY 직접 입력) | 없음 |

### 기능 요구 추적
| 기능 ID | 작업 유형 | 사용자 영향 | 문서 영향 | 완료 기준 |
|---|---|---|---|---|
| F001 | 신규 | 없음(봇 대면) | FC / FRD / ADR | [FRD-001 §17](FRD/SLOTRUNNER-FRD-001.md#17-수용-기준) |
| F002 | 신규 | 슬롯 그리드 가시화 | FC / FRD / ADR | [FRD-002 §17](FRD/SLOTRUNNER-FRD-002.md#17-수용-기준) |
| F003 | 신규 | 없음(자동 구동) | FC / FRD / ADR | [FRD-001 §17](FRD/SLOTRUNNER-FRD-001.md#17-수용-기준) |
| F004 | 신규 | 팝업 결정 | FC / FRD / ADR | [FRD-001 §17](FRD/SLOTRUNNER-FRD-001.md#17-수용-기준) |
| F005 | 신규 | 콘솔 가시화 | FC / FRD | [FRD-002 §17](FRD/SLOTRUNNER-FRD-002.md#17-수용-기준) |
| F006 | 신규 | 슬롯 직접 입력·수동 취소 | FC / FRD / ADR | [FRD-002 §17](FRD/SLOTRUNNER-FRD-002.md#17-수용-기준) |

### 타 App 협력 흐름
> 본 App 은 솔루션 내 유일 App. 외부 결합은 App 이 아닌 외부 자산(Slack봇 agentorchestrator, claudecode-for-me 플러그인, Monday)이다.

| 기능 ID | 협력 대상 | 협력 형태 |
|---|---|---|
| F001 | agentorchestrator(외부 봇) | REST 호출(POST /jobs) |
| F003 | claudecode-for-me 플러그인 / Monday MCP | 슬래시커맨드 주입 / create_update 호출 |

---

## 별도 문서 미작성 항목 안내
- **API Spec**: 각 FRD §17 에 인라인(REST 엔드포인트 계약은 [PIPELINE_ARCHITECTURE.md §6](../../PIPELINE_ARCHITECTURE.md) 초안 보유)
- **UI Spec**: 미작성 (슬롯 그리드·공용 콘솔·EndOfRunModal — FRD §15 에 개념만)
- **Data Spec**: 미작성 (잡 명세 스키마는 PIPELINE_ARCHITECTURE.md §6)
- **Test Case**: forge-scope contract-TDD 단계에서 작성

---

## 확장 후보 기능 (Backlog)
| 기능 ID | 기능명 | 설명 | 상태 | 우선순위 | 근거 |
|---|---|---|---|---|---|
| F101 | N=4 슬롯 확장 | 동시 슬롯 2→4. 2슬롯 검증 후 | Backlog | P2 | [PRD §3.1](SLOTRUNNER-PRD.md#31-릴리즈-범위-본-app-한정) |
| F102 | 봇 결과 역류(WS/콜백) | 진행/완료를 봇으로 역류. 현재는 Monday 댓글 종점 | Backlog | P2 | [PRD §4](SLOTRUNNER-PRD.md#4-비목표) |
| F103 | docs-add-task 선행 단계 | 파이프라인 앞단에 문서 산출 자동화 | Backlog | P2 | DEVELOPMENT_PIPELINE 정본 |
