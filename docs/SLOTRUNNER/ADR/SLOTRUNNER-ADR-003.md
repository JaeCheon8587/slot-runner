# SLOTRUNNER-ADR-003 — 단계 게이트는 파일 판정, 종료/유지는 사람 결정 모달

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-003 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | 파이프라인 단계 = forge-scope→ddr-loop→Monday 통지. 각 단계 산출물은 워크트리 파일 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](../SLOTRUNNER-ARCHITECTURE.md) · [FRD 폴더](../FRD/) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 | jaecheon.jeong |

---

## ADR-003: 단계 전이는 산출물 파일로 결정 판정하고, 완료/실패는 사람 결정 모달로 마감한다

- **상태**: Accepted (2026-06-20)
- **우선순위**: P0
- **컨텍스트**:
  - LLM 세션의 자기보고("다 됐어요")는 신뢰 불가 — 기존 develop-small 도 forge `index.json`·ddr `.review/`로 결정적 판정.
  - 참고 샘플 sidabari 안전 정책: 이벤트 수신만으로 모달 자동 표시 금지, 자동 재시도 금지(사람 결정).
- **결정**:
  - 단계 전이 게이트는 **워크트리 산출물 파일**로 판정한다: forge 완료 = `index.json` 전 step `completed`, ddr 완료 = `.review/<stem>-review.md` 존재. 미달이면 멈춤(다음 단계 주입 안 함).
  - 완료/실패 시 **EndOfRunModal**(사람 결정 게이트)을 띄워 [세션 종료]/[유지]를 받는다. 이는 "자동 모달 금지" 정책의 **의도적 예외**(게이트 모달과 동일 성격).
  - 실패 시 자동 재시도 없음 — 사람이 결정.
- **결과**:
  - 가능: LLM 비의존 결정적 단계 전이, 사람 통제 종료/유지.
  - 제약: 산출물 경로가 스킬 버전에 의존(forge/ddr 산출 규약). 경로 변경 시 게이트 갱신 필요.
- **대안 검토**:
  - 옵션 A (세션 자기보고로 전이): 기각 — 신뢰 불가, 기존 설계 원칙 위반.
  - 옵션 B (완료 시 자동 종료/유지 결정): 기각 — 사람 결정 원칙 위반.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §6 S3·S4·§8 안정성
- [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) — F003·F004
- [SLOTRUNNER-FRD-001](../FRD/SLOTRUNNER-FRD-001.md) — §5 기본 흐름·§7 예외
