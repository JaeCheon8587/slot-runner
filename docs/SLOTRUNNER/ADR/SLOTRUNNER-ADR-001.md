# SLOTRUNNER-ADR-001 — 봇↔앱 통신은 동기 REST 인테이크(tiny_http)

> 본 파일은 단일 App(`SLOTRUNNER`)의 개별 ADR 1건. 인덱스/메타는 [`../SLOTRUNNER-ADR-CATALOG.md`](../SLOTRUNNER-ADR-CATALOG.md) SSOT.

| 항목 | 값 |
|---|---|
| 문서 ID | SLOTRUNNER-ADR-001 |
| 버전 | 0.1 (Draft) |
| 상태 | Accepted |
| 작성 가정 | SlotRunner 는 상시 ON 데스크톱앱. 봇(agentorchestrator)은 별도 프로세스, 같은 머신 |
| 관련 문서 | [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) · [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) · [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) · [SLOTRUNNER-ARCHITECTURE](../SLOTRUNNER-ARCHITECTURE.md) · [FRD 폴더](../FRD/) · [DOCUMENT_GUIDE](../../DOCUMENT_GUIDE.md) |

## 변경 이력
| 버전 | 일자 | 변경 요약 | 작성자 |
|---|---|---|---|
| 0.1 | 2026-06-20 | 초안 | jaecheon.jeong |

---

## ADR-001: 봇은 필요 시에만 앱의 로컬 REST 서버에 잡을 POST 한다 (동기 인테이크, tiny_http)

- **상태**: Accepted (2026-06-20)
- **우선순위**: P0
- **컨텍스트**:
  - 외부 프로세스(봇)는 Tauri IPC(웹뷰↔Rust 내부 전용)를 직접 호출할 수 없다. 같은 머신 채널 필요.
  - 봇이 앱을 실행시키지 않는다. 앱은 상시 ON, 봇은 "필요할 때만" 요청.
  - 잡 1건은 길다(forge+ddr 최대 ~1시간) — 요청이 처리 완료까지 블로킹하면 안 됨.
- **결정**:
  - 앱은 **127.0.0.1 바인드 로컬 REST 서버**를 백그라운드 스레드로 상시 호스팅한다.
  - 봇은 HTTP 클라이언트로 필요 시 `POST /jobs` 한다. 접수 즉시 비동기 수락(202 + job_id), 처리 결과는 Monday 댓글로 통지(봇 역류 없음).
  - REST 구현은 **tiny_http**(동기, 자체 스레드). 비동기 런타임 미도입.
- **결과**:
  - 가능: 봇·앱 독립 상시 프로세스, 느슨한 결합. 상시연결 부담 없음.
  - 제약: 진행 상태를 봇이 알려면 별도 GET 폴링(선택). 신규 의존 1개(tiny_http) + 신규 네트워크 위협면 → localhost 전용 바인드·입력 검증 필수.
- **대안 검토**:
  - 옵션 A (WebSocket 상시연결): 기각 — "필요 시에만 요청" 요구와 불일치, 상시연결 관리 부담.
  - 옵션 B (axum + tokio): 기각 — 참고 샘플 sidabari 가 tokio 를 의도적으로 제거했고, 단순 잡 인테이크에 비동기 런타임은 과함.
  - 옵션 C (파일 드롭 인박스): 기각 — "서버가 돼야 한다"는 요구와 불일치(앱이 능동 리스너여야 함).

### 문서 반영
- [SLOTRUNNER-ADR-CATALOG](../SLOTRUNNER-ADR-CATALOG.md) — Accepted 행 추가
- [SLOTRUNNER-PRD](../SLOTRUNNER-PRD.md) — §8 보안(127.0.0.1)
- [SLOTRUNNER-FC](../SLOTRUNNER-FC.md) — F001
- [SLOTRUNNER-FRD-001](../FRD/SLOTRUNNER-FRD-001.md) — §9 입출력·§13 보안
