# SLOTRUNNER-ADR-008 — 스톨·완료 통지는 데스크톱 토스트, 자동 복구(넛지) 없음

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-008 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | 운영자가 앱을 백그라운드로 띄워둔다. claude 세션은 입력 대기(AskUserQuestion·권한·idle)로 멈출 수 있다. sidabari4loop 는 idle Notification 으로 자동 넛지(stall recovery)함 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ADR-005](SLOTRUNNER-ADR-005.md) · [FRD-004](../FRD/SLOTRUNNER-FRD-004.md) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-21 | 초안 — 토스트 통지·자동복구 배제 결정 본문화 | jaecheon.jeong |

---

## ADR-008: claude 입력 대기·완료/실패는 데스크톱 토스트로 운영자에게 통지하고, 스톨 시 자동 넛지(재주입)는 하지 않는다 — 사람의 선택권을 보존한다

- **상태**: Accepted (2026-06-21)
- **우선순위**: P1
- **컨텍스트**:
  - 슬롯 스텝 루프는 Stop 신호로 진행한다. claude 가 입력 대기(AskUserQuestion·권한 프롬프트·idle)로 멈추면 Stop 이 안 와 **슬롯이 행(hang)** 한다.
  - Claude Code 는 이때 `Notification` 훅을 발화한다.
  - sidabari4loop 는 이 신호로 "묻지 말고 진행해"를 **자동 재주입(스톨 복구)** 한다.
  - 그러나 SlotRunner 철학은 **사람 결정 게이트**(완료/유지 모달 [ADR-003], 수동 개입=슬롯 PTY 직접 입력 [ADR-005]). 자동 넛지는 운영자의 **선택권을 박탈**(이미 진행돼버림)한다.
- **결정**:
  - **통지(토스트)만 한다**: `Notification` 훅(입력 대기 등) → 데스크톱 토스트로 "slot-N 입력 대기/알림". 슬롯 완료/실패(outcome) 도 토스트.
  - **자동 복구(넛지) 안 함**: 멈춘 슬롯에 자동으로 입력을 주입하지 않는다. 운영자가 **슬롯 PTY 직접 입력**([ADR-005] F006)으로 답하거나 종료를 결정한다.
  - 토스트 권한은 1회 요청, 거부 시 생략(콘솔 미러는 유지). 실패는 비차단.
  - 자동 재시도 없음 정책([ADR-004]) 일관.
- **결과**:
  - 가능: 백그라운드 운영 중에도 개입 시점 인지, 운영자 선택권 보존(자율 진행 강제 안 함).
  - 제약: 멈춘 슬롯은 사람이 올 때까지 대기(행 유지) — 의도된 트레이드오프. 무인 자동 진행이 필요하면 후속 ADR 로 옵션(하이브리드) 재논의.
- **대안 검토**:
  - 옵션 A (자동 넛지=sidabari 식): 기각 — 사람 선택권 박탈, SlotRunner 결정 게이트 철학과 충돌.
  - 옵션 C (하이브리드: 토스트 후 T초 무응답 시 자동 넛지): 보류 — 필요 시 후속 ADR.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §3.1 범위·§7 F008·§8 비기능(가용성)
- [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) — F008 등재
- [SLOTRUNNER-FRD-004](../FRD/SLOTRUNNER-FRD-004.md) — 본문
