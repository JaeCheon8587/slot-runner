# SLOTRUNNER-PRD — SlotRunner

> 본 App 의 product 요구사항(시점 = 솔루션 내 단일 App 포커스). 기술 시야(호스트/런타임/진입점) = [`SLOTRUNNER-ARCHITECTURE.md`](SLOTRUNNER-ARCHITECTURE.md). 기능 정의 SSOT = [`SLOTRUNNER-FC.md`](SLOTRUNNER-FC.md).
> **Single-S/W 솔루션** — 본 App PRD 가 솔루션 PRD 역할을 겸유한다.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-PRD |
| 버전 | 0.1 (Draft) |
| 작성 가정 | sidabari4loop(참고 샘플)의 단일 PTY 세션 드라이버를 N슬롯 풀 + 상시 REST 서버로 확장. 봇·플러그인은 외부 결합 |
| 관련 문서 | [SLOTRUNNER-FC](SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](SLOTRUNNER-ARCHITECTURE.md) · [SLOTRUNNER-ADR-CATALOG](SLOTRUNNER-ADR-CATALOG.md) · [/CLAUDE.md](../../CLAUDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 | jaecheon.jeong |
| 0.2 | 2026-06-21 | 구현 후 SSOT 정합 — F007(컨텍스트 자동 압축)·고아 방지(ADR-006)·컨텍스트 압축(ADR-007) 반영 | jaecheon.jeong |
| 0.3 | 2026-06-21 | F008(데스크톱 토스트)·ADR-008(자동복구 배제) 반영 | jaecheon.jeong |

---

## 1. 배경
- Monday 작업 명령을 Slack 봇이 받아 로컬에서 Claude Code 파이프라인(docs-add-task·forge-scope·ddr-loop)을 돌리고 결과를 Monday 댓글로 돌려주는 자동화가 필요하다.
- 기존: 파이썬 중간 래퍼(develop-small/task-add-loop)가 단계마다 `claude -p` 헤드리스 세션을 새로 spawn. 관리 포인트 분산·세션 비재사용.
- SlotRunner 는 상시 켜두는 데스크톱앱이자 REST 서버로서, 영속 Claude Code 세션을 슬롯 풀로 보유해 파이프라인을 직접 구동한다.

## 2. 문제 정의
- 봇(외부 프로세스)이 어떻게 로컬 Claude Code 세션을 트리거하나? (앱을 켜지 않고, 필요할 때만 요청)
- 단계마다 새 프로세스 spawn 없이 영속 세션 1개를 어떻게 단계 게이트와 함께 재사용하나?
- 동시 다수 작업을 어떻게 격리·직렬·대기시키나?
- 완료/실패 시 사람이 세션 종료/유지를 어떻게 결정하나?

## 3. 목표
- 봇 요청을 받는 **상시 로컬 REST 서버**.
- 작업당 **격리된 Claude Code 세션**(슬롯)에서 파이프라인 직접 구동(파이썬 래퍼 제거).
- 단계 전이의 **결정적 게이트**(forge index.json / ddr .review)로 LLM 자기보고 비의존.
- 완료/실패 **팝업 + 세션 종료/유지** 사람 결정.

### 3.1 릴리즈 범위 (본 App 한정)
| 구분 | 범위 |
|---|---|
| MVP 필수 | REST 인테이크(POST /jobs) + 단일 슬롯 영속 세션에 forge-scope→ddr-loop→Monday 통지 주입 + 결정적 게이트 + 단계 타임아웃(7200s) + EndOfRunModal(종료/유지) |
| 이번 릴리즈 포함 | 슬롯 풀 N=2 + FIFO 큐(상한 10·초과 거부+Monday 통지·큐 비우기 API) + 공용 Hook 콘솔(panel_id 라우팅) + 슬롯 PTY 직접 입력(수동 개입·취소) + 컨텍스트 자동 압축(점유 임계 기본 40% → /compact, F007/ADR-007) + 데스크톱 토스트 통지(스톨·완료/실패, 자동복구 없음, F008/ADR-008) |
| 이번 릴리즈 제외 | N=4 확장, 봇으로의 결과 역류(WS), 프로그램적 취소 API(수동 PTY 입력으로 대체 — ADR-005), 진행 상태 영속·재시작 복구(휘발 — ADR-004), docs-add-task 자동 선행 단계(현재는 forge-scope 입력 doc 준비 가정). [SLOTRUNNER-FC Backlog](SLOTRUNNER-FC.md) 등재 |

## 4. 비목표
- 봇·플러그인 자체 개발(외부 자산. SlotRunner 는 결합만).
- 자격증명 저장(경로·ID·프롬프트만 다룬다).
- 헤드리스(무 GUI) 서버 운영 — 본 App 은 데스크톱앱(창이 떠 있어야 함). 헤드리스 경로는 기존 agentorchestrator 가 담당.

## 5. 사용자 / 이해관계자
| 구분 | 역할 | 관심사 |
|---|---|---|
| 운영자(본인 1명) | 앱을 켜두고 슬롯·팝업으로 작업을 감독 | 작업 진행 가시성, 세션 종료/유지 결정, 실패 사유 |
| Slack 봇(agentorchestrator) | REST 클라이언트 | POST /jobs 접수 성공(202), 취소 전달 |

## 6. 핵심 시나리오 (본 App 내부)
| # | 시나리오 | 기대 결과 |
|---|---|---|
| S1 | 봇이 POST /jobs 전송 | 빈 슬롯에 세션 mount → 파이프라인 구동, 202 즉시 반환 |
| S2 | 모든 슬롯 점유 중 추가 잡 | FIFO 큐 적재, 슬롯 비면 자동 dequeue |
| S3 | 파이프라인 완료 | Monday 댓글 등록 + EndOfRunModal(완료) 표시 |
| S4 | 게이트 실패/HALT | Monday 실패 통지 + EndOfRunModal(실패) |
| S5 | 팝업에서 [종료] | 슬롯 unmount(세션 종료, 앱·PTY 생존) → 큐 dequeue |
| S6 | 팝업에서 [유지] | 슬롯 점유 유지(가용 슬롯 감소). 운영자가 나중에 수동 해제 |
| S7 | 큐가 10 인 상태로 추가 잡 도착 | 거부 + 잡의 Monday 링크에 실패 댓글(QUEUE_FULL) |
| S8 | 운영자가 슬롯 선택 후 PTY 직접 입력 | 그 슬롯 세션에 키 전달(수동 개입·취소). 프로그램적 취소 없음 |

> 기능별 상세 흐름은 [`FRD/`](FRD/) 참조.

## 7. 주요 기능 요약 (본 App 한정)
> 본 표는 [`SLOTRUNNER-FC.md`](SLOTRUNNER-FC.md) roll-up. **FC 가 SSOT**.

| 기능 ID | 기능명 | 한 줄 설명 | 릴리즈 범위 |
|---|---|---|---|
| F001 | REST 잡 인테이크 | POST /jobs 접수→검증→큐 적재→202 | MVP |
| F002 | 슬롯 풀 + 큐 | 동시 N(기본 2) 슬롯 배정 + FIFO 큐 | 이번 릴리즈 |
| F003 | 파이프라인 주입·게이트 | 영속 세션에 forge-scope→ddr-loop→Monday 순차 주입, 파일 게이트 판정 | MVP |
| F004 | 종료/유지 팝업 | 완료/실패 EndOfRunModal + 슬롯 종료/유지 | MVP |
| F005 | 공용 Hook 콘솔 | 전 슬롯 이벤트 panel_id 태그 머지 표시 | 이번 릴리즈 |
| F006 | 슬롯 PTY 직접 입력 | 슬롯 선택(포커스) + 키 입력으로 수동 개입·취소 | 이번 릴리즈 |
| F007 | 컨텍스트 자동 압축 | 단계 전이 시 점유율 ≥ 임계(기본 40%)면 /compact 자동 주입 | 이번 릴리즈 |
| F008 | 데스크톱 토스트 통지 | 스톨(입력 대기)·완료/실패 OS 토스트. 자동 복구 없음(사람 결정) | 이번 릴리즈 |

## 8. 비기능 요구사항 (App 특화)
| 분류 | 요구사항 (정량) |
|---|---|
| 보안 | REST 서버는 127.0.0.1 만 바인드. 외부 텍스트(잡 명세·Claude 출력)를 명령으로 실행 금지(데이터로만 취급) |
| 성능 | 잡 접수 응답 < 1s(202, 처리는 비동기). 단계(forge/ddr) 타임아웃 7200s/단계 — [ADR-004](ADR/SLOTRUNNER-ADR-004.md). 컨텍스트 점유 측정은 트랜스크립트 끝부분만 읽고, 임계 미만 시 압축 생략(지연 최소화) — [ADR-007](ADR/SLOTRUNNER-ADR-007.md) |
| 동시성 | 슬롯당 정확히 1 세션. 기본 슬롯 N=2. 큐 상한 10(초과 거부+Monday 통지). 슬롯 간 작업 격리 |
| 안정성 | 자동 재시도 금지(실패 시 멈추고 사람 결정). 단계 판정은 파일 게이트(결정적). 재시작 시 슬롯·큐 휘발(복구 없음) — [ADR-004](ADR/SLOTRUNNER-ADR-004.md). 슬롯 자식 프로세스 트리는 슬롯별 Job Object(KILL_ON_JOB_CLOSE)로 슬롯 해제·앱 종료 시 정리(고아 누적 차단) — [ADR-006](ADR/SLOTRUNNER-ADR-006.md). 컨텍스트 점유 임계(기본 40%) 초과 시 /compact 자동 주입, 압축 무응답 시 무압축 진행(행 방지) — [ADR-007](ADR/SLOTRUNNER-ADR-007.md) |
| 가용성 | 상시 ON 가정. 봇은 필요 시에만 요청(상시 연결 아님). 백그라운드 운영 시 스톨(입력 대기)·완료/실패를 데스크톱 토스트로 통지(자동 복구는 안 함 — 운영자가 슬롯 PTY 직접 입력으로 결정) — [ADR-008](ADR/SLOTRUNNER-ADR-008.md) |

## 9. 제약사항 (App 특화)
- 데스크톱앱(Tauri) — 창이 떠 있어야 동작. 헤드리스 불가.
- 슬롯 = 독립 PTY claude 프로세스 → 동시 N개는 메모리/토큰 N배. 초기 N=2.
- forge-scope/ddr-loop 스킬은 Claude Code 런타임 의존(서브프로세스 0개 불가) → 영속 세션 재사용으로 충족.
- 결정 인용은 [`SLOTRUNNER-ADR-CATALOG`](SLOTRUNNER-ADR-CATALOG.md).

## 10. Feature Catalog / FRD 진입점
| Feature Catalog | 주요 FRD |
|---|---|
| [`SLOTRUNNER-FC`](SLOTRUNNER-FC.md) | [`SLOTRUNNER-FRD-001`](FRD/SLOTRUNNER-FRD-001.md) (예정) |

---

## 부록 A — 요구사항 원본 → 본 App PRD 절 매핑
| 원본 절 | 본 App PRD 절 |
|---|---|
| 작업 목표(단일 루프 파이프라인) | §1·§3 |
| Q1 전달 채널(REST) | §6 S1·§8 보안 |
| Q2 영속 세션 재사용 | §9 |
| Q3 단계 게이트 | §7 F003 |
| Q4 팝업·세션 종료/유지 | §6 S5·S6·§7 F004 |
| Q5 Monday 댓글 | §6 S3·§7 F003 |

## 부록 B — App 사용 errorCode
> 본 App 이 발생시키는 실패 코드. 실패는 EndOfRunModal·콘솔·(해당 시) Monday 댓글에 코드+사유로 노출. SCREAMING_SNAKE.

| errorCode | 발생 기능 | 의미 | 통지 경로 |
|---|---|---|---|
| `JOB_SPEC_INVALID` | F001 | 잡 명세 필수 항목(doc/sln/Monday ID 등) 누락 | REST 4xx |
| `QUEUE_FULL` | F002 | 큐 상한 10 초과로 거부 | Monday 댓글(NotifyPort) |
| `SLOT_SPAWN_FAILED` | F002 | 슬롯 claude 세션 기동 실패 | 콘솔 + Monday 댓글 |
| `FORGE_BLOCKED` | F003 | forge 단계 차단/오류(index.json 미달) | 팝업 + Monday 댓글 |
| `STAGE_TIMEOUT` | F003 | forge/ddr 단계 7200s 초과 | 팝업 + Monday 댓글 |
| `SESSION_DIED` | F003 | 파이프라인 중 PTY 세션 사망 | 팝업 + 콘솔 |
| `MONDAY_NOTIFY_FAILED` | F003 | Monday MCP 미로드/통지 실패 | 팝업 + 콘솔(로컬 결과 보존) |
