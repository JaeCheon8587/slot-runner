# Ports & Adapters (Tauri 적응) 아키텍처 및 코드 작성 규칙 (SlotRunner)

> ⚠ 본 파일은 솔루션의 코드 레이어 규칙 단일 SSOT. 충돌 시 본 문서가 우선한다.
> 원본 양식은 C#/DDD 기준이나, SlotRunner 는 **Tauri 2(Rust 백엔드 + React/TS 프론트)** 스택이라 레이어 모델을 해당 스택에 적응했다.

| 항목 | 값 |
|---|---|
| 문서 ID | ARCHITECTURE (단일 파일) |
| 버전 | 0.1 (Draft) |
| SOLUTION_CODE | SLOTRUNNER |
| 작성 가정 | Ports & Adapters 레이어 모델. 언어 = Rust(src-tauri) + TypeScript/React(src). 진입점 1개 호스트(Tauri 데스크톱앱) |
| 관련 문서 | [DOCUMENT_GUIDE](DOCUMENT_GUIDE.md) · [CLAUDE](../CLAUDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 — Tauri(Rust+React) 적응 레이어 모델 | jaecheon.jeong |

## 1. 문서 목적과 범위

### 1.1 목적
AI 코딩 에이전트와 신규 개발자가 SlotRunner 에 신규 코드를 추가할 때 레이어 위반을 재발시키지 않도록 **반드시** 지켜야 하는 단일 규칙 문서. 모든 규칙은 **반드시**/**허용**/**금지**/**절대 금지** 중 하나로 판정한다.

### 1.2 적용 범위
- 적용 언어: Rust(`src-tauri/src/**`) + TypeScript/React(`src/**`). 진입점 1개(Tauri 데스크톱앱: `src-tauri/src/main.rs` → `lib.rs::run()`).

### 1.3 제외 범위
- 빌드/배포/CI(`vite.config.ts`, `tauri.conf.json`, `Cargo.toml`)는 레이어 룰 적용 외.
- 테스트(`src/**/*.test.ts`, `#[cfg(test)]`)는 §5 매트릭스 적용 외. 단 운영 코드에서 테스트 참조는 **절대 금지**.

## 2. 솔루션 아키텍처

### 2.1 채택 레이어 매핑 (Tauri 적응)

| 표준명 | 솔루션 규약 | 설명 |
|---|---|---|
| Domain | 순수 모델/타입 — Rust `src-tauri/src/domain/**`, TS `src/lib/domain/**` | 잡·슬롯·파이프라인 단계·상태 불변식. 외부 IO 의존 없는 순수 타입 |
| Application | 정책/유스케이스 — Rust `src-tauri/src/app/**`, TS `src/lib/app/**` (또는 store) | 잡 큐·슬롯풀 배정 정책, 파이프라인 단계 전이, Port 인터페이스(trait/interface) |
| Infrastructure | 외부 IO 어댑터 — Rust `src-tauri/src/infra/**` | REST 서버(tiny_http), PTY(portable-pty), 파일 게이트 읽기, hooks_bus(notify), 감사 SQLite |
| Presentation | 진입점·UI — Rust `lib.rs`/`main.rs`/`#[tauri::command]`, React `src/components/**`, `src/App.tsx` | Tauri 조립·IPC 핸들러, 슬롯 그리드·공용 콘솔·EndOfRunModal UI |
| Shared/Cross-Cutting | `src-tauri/src/shared/**`, TS `src/lib/utils.ts` | 공통 상수·유틸·설정 모델 |

> 현행 sidabari4loop(참고 샘플)은 평면 모듈 구조(`pty.rs`/`hooks_bus.rs` 등)다. SlotRunner 는 신규 프로젝트이므로 위 레이어 폴더 규약을 채택해 책임을 분리한다. 단 Tauri `#[tauri::command]` 핸들러와 `run()` 조립은 Presentation 으로 본다.

### 2.2 변형 패턴 (Ports & Adapters)
- 외부 시스템(PTY·파일·HTTP·Monday)은 **반드시** Application 의 Port(trait/interface)로 추상화하고 Infrastructure 가 구현한다.
- Presentation(IPC 핸들러·React)은 Application 정책을 호출한다. Infrastructure 구현체 직접 생성·호출은 조립 루트(`lib.rs::run`)에서만 **허용**.
- Domain 타입은 어느 레이어에서도 참조 **허용**되나, Domain 은 다른 레이어를 **절대 참조 금지**.

## 3. 호스트 책임 분리 — App별 ARCHITECTURE 위임
호스트별 핵심 책임/금지는 [`docs/SLOTRUNNER/SLOTRUNNER-ARCHITECTURE.md`](SLOTRUNNER/SLOTRUNNER-ARCHITECTURE.md) SSOT. 본 문서는 솔루션 공통 룰만 보유. App 코드 후보는 [`/CLAUDE.md` Backend Services Overview](../CLAUDE.md) 단일 출처.

> **충돌 시 우선순위**: 본 솔루션 ARCHITECTURE 룰이 App ARCHITECTURE 보다 우선.

## 4. 레이어별 책임

### 4.1 Domain
- **반드시**: 잡/슬롯/파이프라인 단계/상태 enum·struct·type, 상태 전이 불변식(순수 함수).
- **허용**: 같은 Domain·Shared 참조.
- **금지**: IO(파일/HTTP/PTY/DB) 구현, Tauri/React/외부 crate 호출.
- **절대 금지**: Application·Infrastructure·Presentation 모듈 `use`/`import`.

### 4.2 Application
- **반드시**: 슬롯풀 배정·큐 정책, 파이프라인 단계 전이 로직, Port 인터페이스(예: `PtyInjectPort`, `GateReadPort`, `NotifyPort`, `JobIntakePort`).
- **허용**: Domain·Shared 참조.
- **금지**: 외부 IO 구현체 직접 사용, Tauri command 등록, 구체 UI.
- **절대 금지**: Infrastructure·Presentation 모듈 `use`/`import`.

### 4.3 Infrastructure
- **반드시**: REST 서버(tiny_http), PTY 구동(portable-pty), 게이트 파일 읽기(index.json/.review), hooks_bus(notify), 감사 SQLite. Application Port 구현체.
- **허용**: Application Port·Domain·Shared 참조.
- **금지**: 슬롯풀/파이프라인 정책 결정(=Application 책임)을 구현체에 두기.
- **절대 금지**: Presentation 참조, React 의존.

### 4.4 Presentation
- **반드시**: `main.rs`/`lib.rs::run` 조립·DI, `#[tauri::command]` 핸들러, React UI(슬롯 그리드·공용 콘솔·EndOfRunModal), 이벤트 listen.
- **허용**: Application 호출. Infrastructure 는 조립 루트에서만 **허용**.
- **금지**: 정책·파이프라인 규칙을 UI/핸들러에 직접 소유. 외부 텍스트를 명령으로 실행(보안 — sidabari CLAUDE.md §1.2 계승).
- **절대 금지**: Domain 불변식을 UI 응답 모델로 우회.

### 4.5 Shared/Cross-Cutting
- **반드시**: 공통 상수, 설정 모델(config), 공통 유틸.
- **금지**: Domain 규칙·Application 정책·Infra 어댑터·Presentation 코드.
- **절대 금지**: Domain/Application/Infrastructure/Presentation 참조.

## 5. 레이어 ↔ 아티팩트 카탈로그 (요약)

| 아티팩트 | Domain | Application | Infrastructure | Presentation | Shared |
|---|---|---|---|---|---|
| 모델 타입(Job/Slot/Stage) | **허용** | **금지** | **금지** | **금지** | 공통 값 타입만 **허용** |
| 정책(SlotPool/Queue/Pipeline) | **금지** | **허용** | **금지** | **금지** | **금지** |
| Port(trait/interface) | **금지** | **허용** | **금지** | **금지** | 공통 Port만 **허용** |
| Port 구현(PTY/REST/Gate/Notify) | **금지** | **금지** | **허용** | **금지** | **금지** |
| Tauri command / React 컴포넌트 | **금지** | **금지** | **금지** | **허용** | **금지** |
| Config/Options | 도메인 정책 값만 **허용** | 유스케이스 설정만 **허용** | 외부 시스템 설정만 **허용** | 호스트 바인딩만 **허용** | 공통 설정 모델 **허용** |

## 6. 레이어 참조 방향

### 6.1 절대 금지 매트릭스
명시된 **금지**·**절대 금지** 조합만 위반으로 판정. **허용** 조합은 자유 참조.

| From \ To | Domain | Application | Infrastructure | Presentation | Shared |
|---|---|---|---|---|---|
| Domain | **허용** | **금지** | **금지** | **금지** | **허용** |
| Application | **허용** | **허용** | **금지** | **금지** | **허용** |
| Infrastructure | **허용** | **허용** | **허용** | **금지** | **허용** |
| Presentation | **금지** | **허용** | **허용** | **허용** | **허용** |
| Shared | **금지** | **금지** | **금지** | **금지** | **허용** |

> Presentation→Domain 직접 참조는 **금지**(Application 경유). 단 Domain 의 순수 타입을 IPC 직렬화 DTO 로 노출할 때는 Application 의 Response 타입으로 매핑한다.

## 7. 폴더 → 레이어 자동 판정 표

| 폴더 | 레이어 | 비고 |
|---|---|---|
| `src-tauri/src/domain/**`, `src/lib/domain/**` | Domain | 순수 타입·불변식 |
| `src-tauri/src/app/**`, `src/lib/app/**`, `src/store/**` | Application | 정책·Port·전이. store(Zustand)는 Application 상태로 본다 |
| `src-tauri/src/infra/**` | Infrastructure | REST/PTY/Gate/hooks_bus/audit |
| `src-tauri/src/main.rs`·`lib.rs`, `src-tauri/src/**` 의 `#[tauri::command]`, `src/components/**`, `src/App.tsx` | Presentation | 조립·IPC·UI |
| `src-tauri/src/shared/**`, `src/lib/utils.ts`, `src/lib/config.ts` | Shared | 공통 |
| `**/*.test.ts`, `#[cfg(test)]` | Tests | §5 적용 외. 운영→테스트 참조 **절대 금지** |

### 7.1 접미사 의미 (요약)
- `Port`(trait/interface) = Application 전용. 구현체는 Infrastructure.
- `Controller`/`command` = Presentation 전용(Tauri IPC).
- `Service` = Application(유스케이스 조율) 또는 Infrastructure(외부 래퍼). Domain 은 순수 계산만.
