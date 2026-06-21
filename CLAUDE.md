# 프로젝트: SlotRunner (SLOTRUNNER)

> Monday 명령 → Slack봇 → **SlotRunner(상시 REST 서버)가 N개 슬롯에서 Claude Code 세션을 구동**해 docs-add-task·forge-scope·ddr-loop 파이프라인을 실행 → Monday 댓글까지 도는 단일 루프 오케스트레이터. **모든 설계·결정·기능 명세는 아래 문서들이 단일 진실 공급원(SSOT)**. 코드 작성 전 관련 문서를 직접 읽어 최신 정합성을 확보한다.

## 용어 정의

- **SOLUTION_CODE**: `SLOTRUNNER` — 솔루션(레포 전체) 식별자. 솔루션 공통 문서(`docs/ARCHITECTURE.md` 등)에서 사용.
- **SYSTEM_CODE** ≡ **APP_CODE** ≡ **{App}**: App(S/W 단위) 식별자. 본 파일 § Backend Services Overview 표가 단일 출처(SSOT). 현재 단일 App = `SLOTRUNNER`. App별 문서 ID 패턴 `{SYSTEM_CODE}-PRD`, `{SYSTEM_CODE}-FC`, `{SYSTEM_CODE}-FRD-{NNN}`, `{SYSTEM_CODE}-TASK-{NNN}`, `{SYSTEM_CODE}-ADR-{NNN}`.

상세 식별자 규약은 [`docs/DOCUMENT_GUIDE.md`](docs/DOCUMENT_GUIDE.md) §5 참조.

> **단일 App 솔루션**: SOLUTION_CODE 와 App 코드가 동일(`SLOTRUNNER`). 솔루션 단일 PRD(`docs/PRD.md`)는 미배치 — App PRD 가 cross-cutting 부록을 겸유한다.

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 — SLOTRUNNER 솔루션 부트스트랩, App 1종 등록 | jaecheon.jeong |

## 설계 문서 인덱스

| 영역 | 경로 | 역할 |
|---|---|---|
| **AI 진입점 (본 파일)** | `/CLAUDE.md` | SOLUTION_CODE / SYSTEM_CODE SSOT · Backend Services Overview · 라우터 |
| **문서 작성 룰** | [`docs/DOCUMENT_GUIDE.md`](docs/DOCUMENT_GUIDE.md) | 문서 작성 SSOT — 식별자/메타/변경 이력/SSOT 인용 패턴/AI 작업 시나리오 |
| **솔루션 ARCHITECTURE** | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | 솔루션 공통 룰 (레이어 모델·참조 매트릭스·폴더→레이어 매핑·접미사) — Tauri(Rust+React) 적응 |
| **파이프라인 설계 메모** | [`PIPELINE_ARCHITECTURE.md`](PIPELINE_ARCHITECTURE.md) | 슬롯풀 REST 오케스트레이터 의사결정 기록(설계 단계). 정식 SSOT 는 docs/SLOTRUNNER/ |
| **App: SLOTRUNNER** | [`docs/SLOTRUNNER/`](docs/SLOTRUNNER/) | App별 PRD/FC/ARCHITECTURE/FRD/TASK/ADR/ADR-CATALOG SSOT 폴더 |
| **빈 템플릿 (보존)** | [`docs/.templates/`](docs/.templates/) | Active 양식 + 가이드 원본 |
| Forge/CI 자동화 | 해당 없음 (현재) | — |

폴더 구조 상세는 [`docs/DOCUMENT_GUIDE.md`](docs/DOCUMENT_GUIDE.md) §1.

## Backend Services Overview

본 솔루션의 App 레지스트리. **SYSTEM_CODE 단일 출처(SSOT)**. 신규 App 도입 시 본 표 행 추가가 모든 다른 작업(PRD/FC/FRD/TASK/ADR 작성)보다 선행. App 다수 시 행 복제.

| SYSTEM_CODE | 한 줄 설명 | 호스트 종류 | TFM/런타임 | 폴더 |
|---|---|---|---|---|
| SLOTRUNNER | Monday→Slack→파이프라인을 N슬롯 Claude Code 세션으로 구동하는 상시 REST 오케스트레이터 데스크톱앱 | Tauri 2 데스크톱앱 | Rust (src-tauri) + React 19/TypeScript (Vite) | [`docs/SLOTRUNNER/`](docs/SLOTRUNNER/) |

## 진입 순서

- 신규 작성자/AI 는 [`docs/DOCUMENT_GUIDE.md`](docs/DOCUMENT_GUIDE.md) 를 먼저 읽는다 (작성 룰·식별자·SSOT 인용 패턴).
- 코드 작성 전 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) 절대 금지 매트릭스 확인.
- **신규 기능 작성 흐름** ([DOCUMENT_GUIDE §2](docs/DOCUMENT_GUIDE.md#2-작성-순서) 준수):
  1. `docs/SLOTRUNNER/SLOTRUNNER-PRD.md` §3.1·§7 갱신
  2. `docs/SLOTRUNNER/SLOTRUNNER-FC.md` 5축 표 행 추가
  3. `docs/SLOTRUNNER/FRD/SLOTRUNNER-FRD-{NNN}.md` 신규 (`.templates/App/FRD/APP-FRD-001-TEMPLATE.md` 복사·채움. 코드 상세 금지)
  4. 필요 시 `docs/SLOTRUNNER/ADR/SLOTRUNNER-ADR-{NNN}.md` 등재 + `SLOTRUNNER-ADR-CATALOG.md` 동기화
  5. 구현 착수 전 최신 코드 기준 세부 설계 판단
- **AI 실행용 작업 지시서(TASK) 흐름** — feature/refactor/maintenance/migration/setup/investigation 통합:
  1. (사전) 영향 영구 SSOT(PRD/FC/FRD/ADR/ADR-CATALOG/ARCHITECTURE)를 작성자가 직접 갱신
  2. `docs/SLOTRUNNER/TASK/SLOTRUNNER-TASK-{NNN}.md` 신규 (휘발성·self-contained, 외부 SSOT 인용 금지)
  3. TASK §6 영향 SSOT 표에 갱신 상태 = "완료" 텍스트 선언
  4. TASK §12 컨텍스트 임베드 — 코드 실행에 필요한 계약·구조·정책 본문 임베드
  5. AI 에게 TASK 던져 §8 실행. AI 는 코드만 변경
- **신규 App 추가**: 본 파일 § Backend Services Overview 행 추가 → § 설계 문서 인덱스 행 복제 → `docs/{App}/` + `FRD/`·`ADR/`·`TASK/` 생성 → `.templates/App/` 4종 복사·rename.

## 개발 파이프라인

본 솔루션은 claudecode-for-me 플러그인 파이프라인을 **자기 자신에게 적용(dogfooding)** 해 개발한다:
`grill-me → acceptance-design → meta-prompter → docs-add-task → forge-scope → doc-driven-review/ddr-loop → 반영`.
플러그인 정본: `~/.claude/plugins/cache/claudecode-for-me/.../docs/DEVELOPMENT_PIPELINE.md`.

## 절대 변경 금지

- `docs/.templates/**` — 원본 양식. 사용자 승인 전 수정 금지.
- `docs/DOCUMENT_GUIDE.md`, `docs/ARCHITECTURE.md` — 가이드 SSOT. 사용자 승인 전 수정 금지.
- `/CLAUDE.md`(본 파일), `MEMORY.md` — 사용자 승인 전 수정 금지.
- `agentorchestrator/`, `sidabari4loop-main/` — **참고용 샘플. 수정·삭제 금지** (읽기 전용 레퍼런스).
