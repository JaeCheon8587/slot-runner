# SlotRunner (SLOTRUNNER)

> Monday 작업 명령을 Slack 봇이 받아 로컬 Claude Code 파이프라인을 돌리고 결과를 Monday 댓글로 돌려주는 자동화의 실행 엔진 — **상시 켜두는 데스크톱앱이자 REST 서버로서, 영속 Claude Code 세션을 N개 슬롯 풀로 보유해 파이프라인을 직접 구동**한다.

[![Status](https://img.shields.io/badge/status-Draft-blue)]() [![License](https://img.shields.io/badge/license-Proprietary-green)]()

---

## 개요

SlotRunner 는 `Monday → Slack 봇 → 로컬 파이프라인 → Monday 댓글` 단일 루프의 실행 단계를 담당하는 Tauri 2 데스크톱앱이다. 봇이 보낸 잡(`POST /jobs`)을 빈 슬롯에 mount 해, 슬롯마다 독립된 영속 PTY Claude Code 세션에서 `forge-scope → ddr-loop → Monday 통지` 파이프라인을 순차 주입·구동한다. 단계 전이는 LLM 자기보고가 아니라 **파일 게이트**(forge `index.json` / ddr `.review`)로 결정적으로 판정하고, 완료/실패 시 팝업으로 세션 종료/유지를 사람이 결정한다.

기존 파이썬 래퍼가 단계마다 `claude -p` 헤드리스 세션을 새로 spawn 하던 방식을, **영속 세션을 슬롯 풀로 재사용**하는 방식으로 대체한다.

더 깊은 컨텍스트는 [`docs/SLOTRUNNER/SLOTRUNNER-PRD.md`](docs/SLOTRUNNER/SLOTRUNNER-PRD.md) 참조.

## 빠른 시작

### 사전 요구사항

- **Rust** (stable, edition 2021) + Tauri 2 빌드 의존성 (Windows: MSVC 빌드 툴 / WebView2)
- **Node.js 18+** 및 npm
- **Claude Code CLI** — 슬롯 세션 런타임 (PATH 에 `claude`)
- **OS**: Windows 10/11 (주 개발·검증 환경). 슬롯 자식 프로세스 정리는 Windows Job Object 사용.

### 빌드·실행

프론트엔드 작업 디렉터리는 `Src/SLOTRUNNER/` 다.

```sh
cd Src/SLOTRUNNER

npm install            # 의존성 설치

npm run tauri dev      # 데스크톱앱 개발 모드 (Rust + Vite HMR)
npm run tauri build    # 배포 번들 빌드

npm run dev            # 프론트만 Vite 개발 서버
npm run build          # 프론트 타입체크 + Vite 빌드 (tsc && vite build)
npm test               # 프론트 단위 테스트 (vitest run)
```

Rust 측 단독 작업 시:

```sh
cd Src/SLOTRUNNER/src-tauri
cargo build
cargo test
```

### REST 엔드포인트

REST 서버는 **127.0.0.1 만 바인드**한다 (외부 인터페이스 바인드 금지).

| 메서드·경로 | 용도 |
|---|---|
| `POST /jobs` | 잡 접수 → 검증 → 큐 적재 → 즉시 `202` (큐 상한 10 초과 시 거부 + Monday 통지) |
| `POST /jobs/queue:clear` | 대기 큐 전체 비우기 (실행 중 슬롯 무관) |
| `GET /health` | 헬스 체크 |
| `GET /jobs/{id}` | (선택) 잡 상태 조회 |

> **취소 엔드포인트 없음** — 수동 개입·취소는 운영자가 슬롯 포커스 후 PTY 직접 키 입력으로 수행한다.

봇은 잡 명세에 프로젝트 **논리명(project)** + 루틴(stages) + Monday ID 만 보낸다. 실제 경로(cwd/sln/app)는 SlotRunner 의 프로젝트 레지스트리(`projects.json`)가 소유한다.

## 아키텍처

Tauri 2 단일 윈도우 데스크톱앱. **Rust(src-tauri)** 가 백그라운드 스레드로 127.0.0.1 REST 서버(tiny_http)를 호스팅하고 슬롯별 영속 PTY(portable-pty) Claude Code 세션을 mount/unmount 하며, **React 19/TypeScript(Vite)** 가 N슬롯 그리드 + 공용 Hook 콘솔 UI 를 그린다. 상태(슬롯·큐)는 메모리 보유로 재시작 시 휘발(영속화·복구 없음).

| 핵심 동작 | 설명 |
|---|---|
| 슬롯 풀 | 기본 N=2. 슬롯당 정확히 1 영속 세션. 슬롯 간 작업 격리. |
| 큐 | FIFO, 상한 10. 초과 거부 + Monday 통지(`QUEUE_FULL`). |
| 파이프라인 단계 | `Prep → Forge →(gateF)→ Ddr →(gateD)→ Monday → Done \| Fail` |
| 결정적 게이트 | forge `index.json` step completed / ddr `.review/<stem>-review.md` 존재로 판정 |
| 단계 타임아웃 | forge/ddr 각 7200s, 초과 시 프로세스 트리 종료 + 실패 |
| 고아 방지 | 슬롯 자식 프로세스 트리를 슬롯별 Job Object(KILL_ON_JOB_CLOSE)로 정리 |
| 컨텍스트 자동 압축 | 단계 전이 시 점유율 ≥ 임계(기본 40%)면 `/compact` 자동 주입 |
| 통지 | 완료/실패·스톨을 데스크톱 토스트 + Monday 댓글로. 자동 재시도·복구 없음(사람 결정). |

자세한 호스트 아키텍처는 [`docs/SLOTRUNNER/SLOTRUNNER-ARCHITECTURE.md`](docs/SLOTRUNNER/SLOTRUNNER-ARCHITECTURE.md), 솔루션 공통 레이어 규칙은 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) 참조.

### 소스 레이아웃

```
Src/SLOTRUNNER/
├─ src/                      # React 19 / TypeScript (Presentation)
│  ├─ components/            # SlotPanel · PtyTerminal · EndOfRunModal · *Bridge · StageController
│  ├─ lib/                   # jobs · pty · gate · hooks · context · config
│  └─ store/                 # zustand 앱 스토어
└─ src-tauri/                # Rust (Tauri 2)
   └─ src/
      ├─ domain/             # job 등 도메인
      ├─ infra/              # rest_server · pty · gate · hooks_bus · hook_installer · context_usage
      ├─ config.rs
      ├─ lib.rs              # run() 진입
      └─ main.rs
```

## 문서

| 영역 | 경로 |
|---|---|
| AI 진입점 · App 레지스트리 SSOT | [`CLAUDE.md`](CLAUDE.md) |
| 문서 작성 룰 | [`docs/DOCUMENT_GUIDE.md`](docs/DOCUMENT_GUIDE.md) |
| 솔루션 아키텍처 | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| App 요구사항 (PRD) | [`docs/SLOTRUNNER/SLOTRUNNER-PRD.md`](docs/SLOTRUNNER/SLOTRUNNER-PRD.md) |
| App 기능 레지스트리 (FC) | [`docs/SLOTRUNNER/SLOTRUNNER-FC.md`](docs/SLOTRUNNER/SLOTRUNNER-FC.md) |
| App 호스트 아키텍처 | [`docs/SLOTRUNNER/SLOTRUNNER-ARCHITECTURE.md`](docs/SLOTRUNNER/SLOTRUNNER-ARCHITECTURE.md) |
| App 기능별 상세 (FRD) | [`docs/SLOTRUNNER/FRD/`](docs/SLOTRUNNER/FRD/) |
| App AI 실행용 작업 지시서 (TASK, 휘발성) | [`docs/SLOTRUNNER/TASK/`](docs/SLOTRUNNER/TASK/) |
| App 결정 이력 (ADR) | [`docs/SLOTRUNNER/SLOTRUNNER-ADR-CATALOG.md`](docs/SLOTRUNNER/SLOTRUNNER-ADR-CATALOG.md) |
| 파이프라인 설계 메모 | [`PIPELINE_ARCHITECTURE.md`](PIPELINE_ARCHITECTURE.md) |

> AI 에이전트(Claude Code 등)로 작업 시: 레포 루트 [`CLAUDE.md`](CLAUDE.md) 를 먼저 참조한다 (SOLUTION_CODE / SYSTEM_CODE SSOT · 진입 순서 · 절대 변경 금지 목록).

## 기여

본 프로젝트는 claudecode-for-me 플러그인 파이프라인을 **자기 자신에게 적용(dogfooding)** 하며 개발한다: `grill-me → acceptance-design → meta-prompter → docs-add-task → forge-scope → ddr-loop → 반영`.

- **신규 기능 흐름**: PRD §3.1·§7 → FC 5축 표 행 추가 → `FRD/SLOTRUNNER-FRD-{NNN}` 신규 → (필요 시) ADR 등재 → 구현. FRD 에는 코드 상세를 쓰지 않는다.
- **AI 실행용 코드 작업**(feature / refactor / maintenance / migration / setup / investigation): `TASK/SLOTRUNNER-TASK-{NNN}` 양식(휘발성 + self-contained)으로 작성. 상세는 [`docs/DOCUMENT_GUIDE.md` §2](docs/DOCUMENT_GUIDE.md).
- **커밋 메시지**: Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:` …).
- **절대 변경 금지**: `docs/.templates/**`, `docs/DOCUMENT_GUIDE.md`, `docs/ARCHITECTURE.md`, `CLAUDE.md`, `MEMORY.md`, `agentorchestrator/`, `sidabari4loop-main/` (사용자 승인 전).

## 라이선스

Proprietary (사내). 별도 `LICENSE` 파일 미배치.

## 연락처

- 유지보수자: jaecheon.jeong (jaecheon.jeong@mirero.co.kr)
