import { useEffect, useRef } from "react";
import { listenHookEvent } from "@/lib/hooks";
import { stageDirective, stageLabel, effectiveStages, DONE } from "@/lib/jobs";
import { ptyPaste, ptyEnter } from "@/lib/pty";
import { getContextUsage, contextUsagePct } from "@/lib/context";
import { useAppStore } from "@/store/useAppStore";

// 스텝 루프 컨트롤러 (Supervisor 패턴, 잡별 가변 routine). App 루트 1회 마운트.
// 단계 1 은 PtyTerminal 이 claude 기동 후 주입. 단계 2~ 는 여기서 Stop훅마다 전이·주입.
//   Stop(slot, busy, !outcome) → advanceStage(job.stages 기준) →
//     done → setSlotOutcome(done) (EndOfRunModal)
//     그 외 → [컨텍스트 점유 판정] → 다음 단계 지시 주입(브래킷 페이스트)
//
// 컨텍스트 압축 (sidabari4loop 참고):
//   다음 스텝 주입 전, 트랜스크립트 점유율(pct)을 측정한다.
//   pct ≥ compactThresholdPct → /compact 먼저 주입하고 SessionStart(압축 완료) 대기 후 다음 스텝 주입.
//   pct < 임계 → 바로 다음 스텝 주입(불필요한 느린 압축 회피).
//   측정 불가(transcript 없음/파싱 실패) → 보수적으로 압축.
// 전제: target repo 훅 설치(hook_installer) → Stop·SessionStart 이벤트 발화.
const INJECT_ENTER_DELAY = 800;
// /compact 는 LLM 요약이라 오래 걸린다. 이 한도 내 SessionStart(압축 완료)가 없으면
// 행(hang) 방지를 위해 압축 없이 다음 스텝을 주입하고 경고한다(자동 재시도 안 함).
const COMPACT_WAIT_MS = 600_000; // 10분

type Pending = { directive: string; stage: string };

export function StageController() {
  // 슬롯별 "압축 후 주입 대기" 상태 — listener 가 최신값을 읽도록 ref.
  const pendingRef = useRef<Map<string, Pending>>(new Map());
  const timersRef = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    const timers = timersRef.current;
    const pending = pendingRef.current;

    function clearTimer(slotId: string) {
      const t = timers.get(slotId);
      if (t !== undefined) {
        window.clearTimeout(t);
        timers.delete(slotId);
      }
    }

    // 다음 스텝 지시 1개 주입(브래킷 페이스트 후 Enter).
    function injectDirective(slotId: string, directive: string, stage: string) {
      const { addEvent } = useAppStore.getState();
      ptyPaste(slotId, directive)
        .then(() => setTimeout(() => ptyEnter(slotId), INJECT_ENTER_DELAY))
        .catch((err) => addEvent("SYSTEM", `${slotId} ${stage} 주입 실패: ${err}`));
    }

    async function handleStop(slotId: string, transcriptPath?: string) {
      const { slots, advanceStage, setSlotOutcome, addEvent, compactThresholdPct, contextWindowTokens } =
        useAppStore.getState();
      const slot = slots.find((s) => s.id === slotId);
      if (!slot || slot.status !== "busy" || !slot.job || slot.outcome) return;
      // 이전 압축 대기 잔재 정리(정상 Stop 이 오면 압축 흐름은 종료된 것).
      pending.delete(slotId);
      clearTimer(slotId);

      const stages = effectiveStages(slot.job);
      const prev = stages[slot.stageIndex] ?? DONE;
      const next = advanceStage(slotId);
      addEvent("JOB", `${slotId} 스텝 종료(${stageLabel(prev)}) → 다음: ${stageLabel(next)}`);

      if (next === DONE) {
        setSlotOutcome(slotId, "done", `루틴 완료 (${stages.map(stageLabel).join("→")})`);
        return;
      }
      const directive = stageDirective(slot.job, next);
      if (!directive) return;

      // 컨텍스트 점유 측정 → 임계 미만이면 압축 생략.
      let pct: number | null = null;
      if (transcriptPath) {
        try {
          pct = contextUsagePct(await getContextUsage(transcriptPath, contextWindowTokens));
        } catch {
          pct = null;
        }
      }
      // 측정 후 슬롯 상태가 바뀌었을 수 있어 재확인.
      const cur = useAppStore.getState().slots.find((s) => s.id === slotId);
      if (!cur || cur.status !== "busy" || !cur.job || cur.outcome) return;

      if (pct !== null && pct < compactThresholdPct) {
        addEvent(
          "JOB",
          `${slotId} 컨텍스트 ${pct}% < 임계 ${compactThresholdPct}% — 압축 생략, ${stageLabel(next)} 주입`,
        );
        injectDirective(slotId, directive, next);
        return;
      }

      // 압축 필요 — /compact 먼저, 완료(SessionStart) 후 다음 스텝 주입.
      const pctLabel = pct !== null ? `${pct}%` : "측정불가";
      addEvent(
        "JOB",
        `${slotId} 컨텍스트 ${pctLabel} ≥ 임계 ${compactThresholdPct}% — /compact 후 ${stageLabel(next)} (최대 ${Math.round(COMPACT_WAIT_MS / 60000)}분 대기)`,
      );
      pending.set(slotId, { directive, stage: next });
      injectDirective(slotId, "/compact", "compact");
      clearTimer(slotId);
      timers.set(
        slotId,
        window.setTimeout(() => {
          const p = pending.get(slotId);
          if (!p) return; // 이미 SessionStart 로 처리됨
          pending.delete(slotId);
          timers.delete(slotId);
          useAppStore
            .getState()
            .addEvent("SYSTEM", `${slotId} /compact 후 ${Math.round(COMPACT_WAIT_MS / 60000)}분 내 완료 신호 없음 — 압축 없이 ${stageLabel(p.stage)} 주입`);
          injectDirective(slotId, p.directive, p.stage);
        }, COMPACT_WAIT_MS),
      );
    }

    // 압축 완료(SessionStart) → 대기 중이던 다음 스텝 주입.
    function handleSessionStart(slotId: string, source?: string) {
      const p = pending.get(slotId);
      if (!p) return; // 이 슬롯은 압축 대기 중이 아님 — 일반 SessionStart 무시
      pending.delete(slotId);
      clearTimer(slotId);
      const { slots, addEvent } = useAppStore.getState();
      const slot = slots.find((s) => s.id === slotId);
      if (!slot || slot.status !== "busy" || !slot.job || slot.outcome) return;
      addEvent("JOB", `${slotId} 압축 완료(source=${source ?? "?"}) — ${stageLabel(p.stage)} 주입`);
      injectDirective(slotId, p.directive, p.stage);
    }

    listenHookEvent((e) => {
      if (!e.panel_id) return;
      const tp = e.payload.transcript_path;
      const transcriptPath = typeof tp === "string" ? tp : undefined;
      if (e.kind === "stop") {
        void handleStop(e.panel_id, transcriptPath);
      } else if (e.kind === "session-start") {
        const src = e.payload.source;
        handleSessionStart(e.panel_id, typeof src === "string" ? src : undefined);
      }
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch((err) => console.warn("[StageController] listen 실패:", err));

    return () => {
      cancelled = true;
      unlisten?.();
      timers.forEach((t) => window.clearTimeout(t));
      timers.clear();
      pending.clear();
    };
  }, []);

  return null;
}
