# SLOTRUNNER-ADR-005 — 취소·개입은 슬롯 PTY 직접 입력, REST 취소 없음

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-005 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | 운영자가 앱 앞에서 슬롯을 감독. 각 슬롯은 PTY(xterm) 렌더 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](../SLOTRUNNER-ARCHITECTURE.md) · [FRD 폴더](../FRD/) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 | jaecheon.jeong |

---

## ADR-005: 작업 취소·수동 개입은 운영자가 슬롯 PTY 에 직접 입력해 수행한다 (프로그램적 취소 API 없음)

- **상태**: Accepted (2026-06-20)
- **우선순위**: P1
- **컨텍스트**:
  - 진행중 작업을 멈추거나 방향을 바꿔야 할 때가 있다.
  - 자동 취소 로직은 복잡하고 위험(어디까지 정리? 워크트리?). 사람 결정 원칙(자동 재시도/자동 실행 금지)과 정렬.
- **결정**:
  - **REST 취소 엔드포인트 없음**. 취소·수동 개입은 운영자가 해당 슬롯의 PTY 에 직접 키 입력(예: Esc, Ctrl-C, 지시 텍스트)으로 수행한다.
  - 따라서 UI 는 **슬롯을 개별 선택(포커스)하고 키 입력을 그 슬롯 PTY 로 보낼 수 있어야 한다**(F006).
  - 대기 큐 제거는 별도(큐 비우기 API, [ADR-002](SLOTRUNNER-ADR-002.md) 큐 정책) — 실행중 슬롯과 무관.
- **결과**:
  - 가능: 유연한 사람 개입, 취소 정리 로직 불요.
  - 제약: 봇·자동화로 실행중 작업을 멈출 수 없음(운영자 수동 전제). 큐 항목은 큐 비우기 API 로만 일괄 제거.
- **대안 검토**:
  - 옵션 A (REST `/jobs/{id}/cancel` + 자동 정리): 기각 — 정리 범위 복잡·위험, 사람 결정 원칙.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §3.1·§6 S8
- [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) — F006
- [SLOTRUNNER-FRD-002](../FRD/SLOTRUNNER-FRD-002.md) — §5·§11 (슬롯 포커스·입력)
