import { useEffect } from "react";
import { listenHookEvent } from "@/lib/hooks";
import { stageDirective, stageLabel, effectiveStages, DONE } from "@/lib/jobs";
import { ptyPaste, ptyEnter } from "@/lib/pty";
import { useAppStore } from "@/store/useAppStore";

// 스텝 루프 컨트롤러 (Supervisor 패턴, 잡별 가변 routine). App 루트 1회 마운트.
// 단계 1 은 PtyTerminal 이 claude 기동 후 주입. 단계 2~ 는 여기서 Stop훅마다 전이·주입.
//   Stop(slot, busy, !outcome) → advanceStage(job.stages 기준) →
//     done → setSlotOutcome(done) (EndOfRunModal)
//     그 외 → 다음 단계 지시 주입(브래킷 페이스트)
// 전제: target repo 훅 설치(hook_installer) → Stop 이벤트 발화.
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

      const stages = effectiveStages(slot.job);
      const prev = stages[slot.stageIndex] ?? DONE;
      const next = advanceStage(slot.id);
      addEvent("JOB", `${slot.id} 스텝 종료(${stageLabel(prev)}) → 다음: ${stageLabel(next)}`);

      if (next === DONE) {
        setSlotOutcome(slot.id, "done", `루틴 완료 (${stages.map(stageLabel).join("→")})`);
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
