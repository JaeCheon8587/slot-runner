# SLOTRUNNER-ADR-006 — 슬롯 자식 프로세스 트리는 슬롯별 Job Object 로 정리(고아 방지)

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-006 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | 슬롯 = 독립 PTY claude 프로세스. claude 는 다시 node/dotnet 등 자식 트리를 낳는다. Windows 데스크톱앱 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](../SLOTRUNNER-ARCHITECTURE.md) · [SLOTRUNNER-ADR-004](SLOTRUNNER-ADR-004.md) · [FRD-002](../FRD/SLOTRUNNER-FRD-002.md) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-21 | 초안 — 슬롯별 Job Object 고아 방지 결정 본문화 | jaecheon.jeong |

---

## ADR-006: 슬롯 PTY 자식 트리를 슬롯별 Windows Job Object(KILL_ON_JOB_CLOSE)로 묶어 슬롯 해제·앱 종료 시 트리째 정리한다

- **상태**: Accepted (2026-06-21)
- **우선순위**: P1
- **컨텍스트**:
  - 슬롯 PTY 의 최상위 프로세스(claude / npm shim 래퍼)는 다시 node·dotnet 빌드 등 **자식 트리**를 낳는다.
  - 슬롯 해제 시 호출하는 단일 프로세스 종료(TerminateProcess 1건)는 **최상위만** 죽이고 자손은 고아로 남는다.
  - 앱 크래시 시에도 진행중 자식 트리가 거둬지지 않아 **고아 누적 → 리소스 고갈 → 추가 크래시** 악순환이 관측됐다.
- **결정**:
  - 슬롯 PTY 자식을 **슬롯 id 별 Windows Job Object** 에 할당한다. job 은 `KILL_ON_JOB_CLOSE` 속성.
  - **슬롯 해제(세션 종료)·재기동(remount)**: 그 슬롯 job 핸들만 닫아 → OS 가 자식 트리(claude→node/dotnet)를 **즉시 종료**한다. 앱은 계속 동작.
  - **앱 종료/크래시**: 프로세스 종료 시 OS 가 남은 모든 슬롯 job 핸들을 닫아 → 전체 트리 종료(백스톱).
  - 정리는 best-effort(실패해도 앱 흐름 비차단). 자동 재시도 없음([ADR-004] 정책 계승).
- **결과**:
  - 가능: 슬롯 해제만으로 자손까지 정리(앱 재시작 불요), 크래시 후 고아 누적 차단.
  - 제약: Windows 전용 메커니즘(타 OS 는 no-op — 본 App 은 Windows 데스크톱앱이라 무방).
- **대안 검토**:
  - 옵션 A (단일 전역 Job Object): 기각 — 슬롯 해제 단위 정리 불가(앱 종료 시에만 발동).
  - 옵션 B (해제 시 프로세스 트리 수동 열거·종료, taskkill /T 식): 기각 — 경쟁 조건·수동, OS 보장 약함.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §8 비기능(안정성)
- [SLOTRUNNER-FRD-002](../FRD/SLOTRUNNER-FRD-002.md) — §10 상태(슬롯 종료 시 자식 트리 정리)
