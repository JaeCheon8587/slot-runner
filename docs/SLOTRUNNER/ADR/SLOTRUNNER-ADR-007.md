# SLOTRUNNER-ADR-007 — 스텝 전이 시 컨텍스트 점유 임계 기반 /compact 자동 주입

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-007 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | 스텝 루프는 한 슬롯 claude 세션을 재사용하며 단계마다 지시를 주입(컨텍스트 누적). sidabari4loop 의 컨텍스트 점유 추정·리셋 기법 참고 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](../SLOTRUNNER-ARCHITECTURE.md) · [SLOTRUNNER-ADR-003](SLOTRUNNER-ADR-003.md) · [FRD-003](../FRD/SLOTRUNNER-FRD-003.md) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-21 | 초안 — 컨텍스트 점유 임계 기반 /compact 결정 본문화 | jaecheon.jeong |

---

## ADR-007: 스텝 전이 직전 트랜스크립트로 컨텍스트 점유율을 측정해 임계 이상이면 /compact 를 먼저 주입하고 압축 완료 후 다음 스텝을 잇는다

- **상태**: Accepted (2026-06-21)
- **우선순위**: P1
- **컨텍스트**:
  - 스텝 루프는 한 슬롯 claude 세션을 재사용하며 단계(docs-add-task→forge-scope→ddr-loop 등)마다 지시를 주입한다 → 컨텍스트가 단계마다 **누적**(빌드 로그 등).
  - Claude Code 내장 auto-compact 는 자체 경계에서만 발동 — 앱이 시점/임계를 제어하지 못한다.
  - sidabari4loop 는 트랜스크립트의 마지막 assistant `usage`(input+cache)로 점유를 추정하고 임계 기반으로 /compact·/clear 를 주입한다.
- **결정**:
  - **점유 측정**: Stop(스텝 종료) 시 Hook payload 의 transcript_path 로 트랜스크립트(.jsonl) 끝부분을 읽어 마지막 (비 sidechain) assistant `usage` 합(input + cache_creation + cache_read)을 현재 컨텍스트 점유로 추정. 분모 = 컨텍스트 윈도우 토큰(설정, 기본 1,000,000).
  - **판정**: 점유율 ≥ `compact_threshold_pct`(설정, 기본 **40**) 이면 다음 스텝 주입 **전에 `/compact` 주입** → 압축 완료(SessionStart) 신호 후 다음 스텝 지시 주입. 임계 미만이면 압축 생략·즉시 다음 스텝(불필요한 느린 압축 회피). **측정 불가**(transcript 없음/파싱 실패)면 보수적으로 압축.
  - **행 방지**: /compact 후 일정 한도(기본 10분) 내 완료 신호가 없으면 압축 없이 다음 스텝을 주입하고 경고(자동 재시도 없음 — [ADR-004] 정책 계승).
  - 슬롯별 독립 판정([ADR-002] 격리) — 슬롯마다 자기 트랜스크립트·대기 상태.
- **결과**:
  - 가능: 긴 routine 에서 컨텍스트 폭주 전 선제 압축, 임계 미만 시 압축 생략으로 속도 보존, 임계·윈도우 설정으로 조정.
  - 제약: /compact 는 LLM 요약이라 지연 발생(대기). transcript 미측정 시 과압축 가능(보수적).
- **대안 검토**:
  - 옵션 A (내장 auto-compact 에 전적 위임): 기각 — 시점·임계 제어 불가, 슬롯 단위 운영 가시성 없음.
  - 옵션 B (스텝마다 /clear 로 컨텍스트 리셋): 기각(현재) — 단계 간 맥락 유실 위험. 스텝은 같은 세션 맥락을 잇는 것이 안전.
  - 옵션 C (스텝당 fresh claude 프로세스, sidabari 식 완전 리셋): 보류 — 구조 변경 큼. 필요 시 후속 ADR.

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §3.1 범위·§7 F007·§8 비기능(성능/안정성)
- [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) — F007 등재
- [SLOTRUNNER-FRD-003](../FRD/SLOTRUNNER-FRD-003.md) — 본문
