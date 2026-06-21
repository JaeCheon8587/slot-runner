# SLOTRUNNER-ADR-002 — 처리 모델은 N슬롯 풀 + FIFO 큐 (세션 격리)

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-002 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | 윈도우 1개에 N분할 슬롯 그리드. 각 슬롯 = 독립 PTY claude 세션 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](../SLOTRUNNER-ARCHITECTURE.md) · [FRD 폴더](../FRD/) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 | jaecheon.jeong |

---

## ADR-002: 잡은 N슬롯 풀에서 격리 실행하고 초과분은 FIFO 큐에 둔다 (기본 N=2, 유지=점유)

- **상태**: Accepted (2026-06-20)
- **우선순위**: P1
- **컨텍스트**:
  - 다수 잡이 동시에 올 수 있다. 잡마다 컨텍스트가 섞이면 안 됨(격리 필요).
  - 슬롯 = 독립 PTY claude 프로세스 → 동시 N개는 메모리/토큰 N배.
  - 완료된 잡의 세션을 사람이 더 보고 싶을 수 있다(유지) vs 비워야 할 수도(종료).
- **결정**:
  - 잡당 1개 슬롯 = 1개 영속 PTY claude 세션(격리). 빈 슬롯에 배정.
  - 슬롯 수 초과 잡은 **FIFO 큐** 적재, 슬롯 비면 dequeue.
  - 기본 **N=2**(리소스). `config.slots`로 두고 추후 4까지 확장.
  - 완료/실패 후 [유지] 선택 시 **슬롯 점유 유지**(가용 슬롯 감소). 운영자가 수동 해제. [종료]는 슬롯 unmount(앱·PTY 풀 자체는 생존).
- **결과**:
  - 가능: 동시 N 작업 격리 처리, 초과분 자동 대기.
  - 제약: 유지가 슬롯을 잠식 → 큐 적체 가능(의도된 동작, 사람이 수동 해제로 관리). 동시 N = 리소스 N배.
- **대안 검토**:
  - 옵션 A (단일 PTY 직렬): 기각 — 동시성 없음, 최종 그림(4분할 윈도우)과 불일치.
  - 옵션 B (잡마다 신규 프로세스 spawn, 풀 없음): 기각 — 세션 재사용 의도(파이썬 래퍼 제거)와 충돌, 무제한 동시성 리스크.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §6 S2·S6·§9 제약
- [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) — F002
- [SLOTRUNNER-FRD-002](../FRD/SLOTRUNNER-FRD-002.md) — §8·§10 상태
