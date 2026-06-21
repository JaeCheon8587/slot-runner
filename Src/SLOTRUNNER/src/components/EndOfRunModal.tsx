import { useAppStore } from "@/store/useAppStore";

// 완료/실패 결정 게이트 (F004 / ADR-003). 슬롯에 outcome 이 생기면 표시.
// "이벤트만으로 자동 모달 금지" 정책의 의도적 예외 — 사람 결정 게이트(종료/유지).
// 한 번에 하나(outcome 있는 첫 슬롯). 외부클릭/ESC 자동닫힘 없음 — 반드시 선택.
export function EndOfRunModal() {
  const slot = useAppStore((s) => s.slots.find((x) => x.outcome));
  const releaseSlot = useAppStore((s) => s.releaseSlot);
  const keepSession = useAppStore((s) => s.keepSession);
  const addEvent = useAppStore((s) => s.addEvent);

  if (!slot || !slot.outcome) return null;
  const { kind, reason } = slot.outcome;
  const short = slot.job?.job_id.slice(0, 8) ?? "";

  const onEnd = () => {
    const r = releaseSlot(slot.id);
    addEvent(
      "SYSTEM",
      r.kind === "reassigned"
        ? `${slot.id} 세션 종료 → 큐에서 job=${r.job.job_id.slice(0, 8)} 재배정`
        : `${slot.id} 세션 종료(빈 슬롯)`,
    );
  };
  const onKeep = () => {
    keepSession(slot.id);
    addEvent("SYSTEM", `${slot.id} 세션 유지 (점유 지속 — 수동 해제 전까지 가용 슬롯 감소)`);
  };

  return (
    <div className="modal-overlay">
      <div className="modal">
        <div className={`modal-title ${kind}`}>
          {kind === "done" ? "✅ 완료" : "⚠️ 실패"} — {slot.id} · job {short}
        </div>
        <div className="modal-body">
          <div>phase: {slot.job?.phase ?? "-"}</div>
          {reason ? <div className="modal-reason">{reason}</div> : null}
          <div className="modal-hint">세션을 종료(슬롯 해제)하거나 유지(점유 지속)합니다.</div>
        </div>
        <div className="modal-actions">
          <button className="btn ghost" onClick={onKeep}>
            유지
          </button>
          <button className="btn" onClick={onEnd}>
            세션 종료
          </button>
        </div>
      </div>
    </div>
  );
}
