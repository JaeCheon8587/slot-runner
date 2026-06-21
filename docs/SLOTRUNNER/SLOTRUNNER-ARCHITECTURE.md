# SLOTRUNNER-ARCHITECTURE — SlotRunner 호스트 아키텍처

> App별 ARCHITECTURE. 솔루션 공통 룰은 [솔루션 ARCHITECTURE](../ARCHITECTURE.md) SSOT 우선. 본 문서는 호스트 특이 사항만 보유.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ARCHITECTURE |
| 버전 | 0.1 (Draft) |
| App 코드 | SLOTRUNNER |
| 작성 가정 | 솔루션 공통 룰([../ARCHITECTURE.md](../ARCHITECTURE.md)) 준수. Tauri 2 단일 호스트. sidabari4loop 샘플의 PTY/hooks 패턴 참고(복사 금지) |
| 관련 문서 | [솔루션 ARCHITECTURE](../ARCHITECTURE.md) · [SLOTRUNNER-PRD](SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](SLOTRUNNER-FC.md) · [SLOTRUNNER-ADR-CATALOG](SLOTRUNNER-ADR-CATALOG.md) · [FRD 폴더](FRD/) · [/CLAUDE.md](../../CLAUDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 | jaecheon.jeong |

## 1. App 개요
| 항목 | 값 |
|---|---|
| App 코드 | SLOTRUNNER |
| 한 줄 설명 | 상시 REST 서버 + N슬롯 영속 Claude Code 세션 풀로 파이프라인 구동 |
| TFM/런타임 | Rust(src-tauri) + React 19/TypeScript(Vite), Tauri 2 |
| 진입점 경로 | `src-tauri/src/main.rs` → `src-tauri/src/lib.rs::run()` (프론트 진입 `src/main.tsx` → `src/App.tsx`) |
| 호스트 종류 | Tauri 2 데스크톱앱 (단일 윈도우, N슬롯 그리드 + 공용 콘솔) |

## 2. 핵심 책임 (4단계 마커)
- **반드시** 127.0.0.1 바인드 REST 서버를 백그라운드 스레드로 호스팅하고 잡을 Application 큐로 넘긴다.
- **반드시** 슬롯당 1개 영속 PTY claude 세션을 mount/unmount 하고, 파이프라인 단계를 주입(ptyWrite+브래킷페이스트)한다.
- **반드시** 단계 전이를 파일 게이트(forge `index.json` 전 step completed / ddr `.review/<stem>-review.md` 존재)로 판정한다(자기보고 비의존).
- **허용** Monday MCP 호출은 슬롯의 claude 세션이 직접 수행(create_update). 앱은 ID·본문을 운영프롬프트로 주입만.
- **금지** 자동 재시도. 실패 시 멈추고 EndOfRunModal 로 사람 결정.
- **금지** 외부 텍스트(잡 명세·Claude 출력·Hook payload)를 셸 명령 문자열로 조합·실행. CommandBuilder 개별 인자만.
- **절대 금지** REST 리스너의 외부 인터페이스 바인드(0.0.0.0 등).

## 3. 외부 IO Adapter 위치 정책
- REST 서버(tiny_http) = Infrastructure(`infra/rest_*`). 127.0.0.1 바인드. 엔드포인트: `POST /jobs`(접수, 큐<10 시 적재·≥10 시 거부+Monday 통지), `POST /jobs/queue:clear`(대기 큐 전체 비우기), `GET /health`, (선택)`GET /jobs/{id}`. **취소 엔드포인트 없음** — 수동 PTY 입력([ADR-005](SLOTRUNNER-ADR-005.md)). 수신 잡을 Application `JobIntakePort` 로 전달.
- PTY(portable-pty) = Infrastructure(`infra/pty_*`). Application `PtyInjectPort` 구현. 슬롯별 PTY + 운영자 직접 키 입력(활성 슬롯 대상) 경로.
- 게이트 파일 읽기(index.json/.review) = Infrastructure(`infra/gate_*`). Application `GateReadPort` 구현. 경로 traversal 방어(허용 루트 하위만), sidabari `read_project_text` 패턴 참고.
- Hook 이벤트(notify watcher + events.jsonl tail) = Infrastructure(`infra/hooks_*`). panel_id 로 슬롯 라우팅해 Presentation 으로 emit.
- Monday 쓰기 = Application `NotifyPort`. 2경로: (a) **정상 파이프라인 통지** = 슬롯 claude 세션의 Monday MCP create_update(세션 런타임, 신규 spawn 없음, 앱은 주입 텍스트만 구성). (b) **슬롯 없는 실패 통지**(큐 포화 QUEUE_FULL 등) = 일회성 헤드리스 Monday 통지(슬롯 세션 없음 → 단발 호출). sidabari `post_monday_reply` 패턴 참고.

## 4. App 특이 도메인/패턴 룰
- **슬롯**: `{panel_id(main-1..N), status(empty|mounting|running|popup|kept), job, pty_handle, pipeline_stage}`. panel_id = PTY env `SIDABARI4LOOP_PANEL_ID` 상당(신규 명명, sidabari 와 충돌 회피).
- **파이프라인 단계 머신**: Prep→Forge→(gateF)→Ddr→(gateD)→Monday→Done | Fail. 단계 전이는 Application, 주입·판독은 Infrastructure Port.
- **슬롯 풀 머신**: Empty→Mounting→Running→Popup(Done/Fail)→Unmount(종료)|Kept(유지). Kept = 점유 유지(가용 슬롯 감소, 의도). 활성(포커스)은 직교 속성(운영자 입력 대상).
- **큐**: FIFO, **상한 10**. 초과 거부 + Monday 통지(QUEUE_FULL). 큐 비우기 = 대기 항목 전체 제거(실행중 슬롯 무관).
- **수동 개입/취소**: 운영자가 슬롯 포커스 후 PTY 직접 키 입력 — [ADR-005](SLOTRUNNER-ADR-005.md). 프로그램적 취소 없음.
- **단계 타임아웃**: forge/ddr 각 7200s, 초과 시 프로세스 트리 종료 + 실패(STAGE_TIMEOUT) + 슬롯 해제 — [ADR-004](SLOTRUNNER-ADR-004.md).
- **상태 휘발**: 슬롯·큐는 메모리 보유, 재시작 시 소실(영속화 없음) — [ADR-004](SLOTRUNNER-ADR-004.md).
- **동시성**: 슬롯 N개(기본 2) = 독립 PTY. 큐 초과분 직렬 대기. 슬롯 간 상태 공유 금지(격리).
- **중복 잡 + 워크트리 격리**: 동일 잡(같은 phase) 중복 허용(중복 무시). 따라서 작업 공간(워크트리/브랜치)을 **잡 단위로 격리**해 동시 동일 phase 충돌을 막는다 — `feat-<phase>` 단독 키 금지, 잡 식별자 포함(예: `<phase>-<job_id>`). 기존 develop-small 의 phase 단독 워크트리 키와 다른 점.
- 상세 흐름·다이어그램은 [PIPELINE_ARCHITECTURE.md](../../PIPELINE_ARCHITECTURE.md)(설계 메모) + FRD-001/002.

## 5. 솔루션 SSOT 인용
본 App 작업 시 솔루션 공통 룰 **반드시** 준수:
- [§2 채택 레이어 매핑](../ARCHITECTURE.md#2-솔루션-아키텍처)
- [§4 레이어별 책임](../ARCHITECTURE.md#4-레이어별-책임)
- [§5 카탈로그 매트릭스](../ARCHITECTURE.md#5-레이어--아티팩트-카탈로그-요약)
- [§6.1 절대 금지 매트릭스](../ARCHITECTURE.md#61-절대-금지-매트릭스)
- [§7 폴더 → 레이어 매핑](../ARCHITECTURE.md#7-폴더--레이어-자동-판정-표)

솔루션 룰과 본 App 룰 충돌 시 **솔루션 ARCHITECTURE 우선**.
