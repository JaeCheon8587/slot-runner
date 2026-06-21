import { useEffect } from "react";
import { listenHookEvent } from "@/lib/hooks";
import { stageDirective, STAGE_LABEL } from "@/lib/jobs";
import { ptyPaste, ptyEnter } from "@/lib/pty";
import { useAppStore } from "@/store/useAppStore";

// 스텝 루프 컨트롤러 (Supervisor 패턴). App 루트 1회 마운트.
// 단계 1(docs-add-task)는 PtyTerminal 이 claude 기동 후 주입. 단계 2~3 은 여기서 Stop훅마다 전이·주입.
//   Stop(slot, busy, !outcome) → advanceStage →
//     done  → setSlotOutcome(done) (EndOfRunModal)
//     그 외 → 다음 단계 지시 주입(브래킷 페이스트)
// 전제: target repo 에 훅 설치돼 Stop 이벤트가 발화해야 함(hook_installer).
// 붙여넣기 후 Enter 제출 지연. 동시 다중 세션 부하서 Enter 묻힘 방지(800ms).
const INJECT_ENTER_DELAY = 800;

export function StageController() {
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    listenHookEvent((e) => {
      if (e.kind !== "stop" || !e.panel_id) return;
      const { slots, advanceStage, setSlotOutcome, addEvent } = useAppStore.getState();
      const slot = slots.find((s) => s.id === e.panel_id);
      if (!slot || slot.status !== "busy" || !slot.job || slot.outcome) return;

      const prevStage = slot.stage;
      const next = advanceStage(slot.id);
      addEvent("JOB", `${slot.id} 스텝 종료(${STAGE_LABEL[prevStage]}) → 다음: ${STAGE_LABEL[next]}`);

      if (next === "done") {
        setSlotOutcome(slot.id, "done", "루틴 완료 (docs-add-task→forge-scope→ddr-loop)");
        return;
      }
      const directive = stageDirective(slot.job, next);
      if (!directive) return;
      ptyPaste(slot.id, directive)
        .then(() => setTimeout(() => ptyEnter(slot.id), INJECT_ENTER_DELAY))
        .catch((err) => addEvent("SYSTEM", `${slot.id} ${next} 주입 실패: ${err}`));
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch((err) => console.warn("[StageController] listen 실패:", err));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return null;
}
