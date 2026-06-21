# SLOTRUNNER-ADR-004 — 운영 상태는 휘발, 단계는 타임아웃으로 보호

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-004 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | 상시 ON 데스크톱앱이나 크래시·재시작은 발생 가능. 슬롯=독립 PTY claude 프로세스 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](../SLOTRUNNER-ARCHITECTURE.md) · [FRD 폴더](../FRD/) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 | jaecheon.jeong |

---

## ADR-004: 진행중 슬롯·큐는 영속화하지 않고(휘발), 각 단계는 7200s 타임아웃으로 보호한다

- **상태**: Accepted (2026-06-20)
- **우선순위**: P1
- **컨텍스트**:
  - 앱 재시작/크래시 시 진행중 PTY claude 세션은 자동 거둘 수 없다(재연결 불가, 프로세스 사망).
  - 슬롯은 영구 점유 위험이 있다(세션 멈춤·무한 빌드).
- **결정**:
  - **재시작 복구 없음(휘발)**: 진행중 슬롯·대기 큐는 디스크에 영속화하지 않는다. 재시작 시 소실 → 봇이 재요청한다. (워크트리 산출물은 디스크에 남아 provenance 유지.)
  - **단계 타임아웃 = 7200s/단계**(forge 7200s, ddr 7200s). 초과 시 그 단계 프로세스 트리 종료 + 실패 마감(STAGE_TIMEOUT) + 슬롯 해제 대상.
- **결과**:
  - 가능: 구현 단순(영속 인프라 불요), 슬롯 영구 점유 방지.
  - 제약: 크래시 시 진행 작업 유실(사람이 재요청). 매우 긴 정상 작업이 7200s 넘으면 오탐 → 값 조정 가능(config).
- **대안 검토**:
  - 옵션 A (전체 영속화·재연결): 기각 — PTY claude 재연결 비현실적.
  - 옵션 B (무제한 타임아웃): 기각 — 멈춘 세션이 슬롯 영구 점유.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §8 비기능·§9 제약
- [SLOTRUNNER-FRD-001](../FRD/SLOTRUNNER-FRD-001.md) — §7 예외(E5)·§13
- [SLOTRUNNER-FRD-002](../FRD/SLOTRUNNER-FRD-002.md) — §10 상태(휘발)
