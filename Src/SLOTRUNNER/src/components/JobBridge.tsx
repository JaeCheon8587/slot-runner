import { useEffect } from "react";
import { listenJobRequest, listenQueueClear } from "@/lib/jobs";
import { useAppStore, QUEUE_CAP } from "@/store/useAppStore";

// 백엔드 "job:request"/"queue:clear" 수신 → 슬롯 풀 배정·큐 비우기. App 루트에 1회만 마운트.
export function JobBridge() {
  const addEvent = useAppStore((s) => s.addEvent);
  const assignJob = useAppStore((s) => s.assignJob);
  const clearQueue = useAppStore((s) => s.clearQueue);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];
    const track = (fn: () => void) => {
      if (cancelled) fn();
      else unlisteners.push(fn);
    };

    listenJobRequest((job) => {
      const short = job.job_id.slice(0, 8);
      const r = assignJob(job);
      if (r.kind === "assigned") {
        addEvent("JOB", `job=${short} phase=${job.phase} cwd=${job.cwd} → ${r.slotId} 배정(claude+forge)`);
      } else if (r.kind === "queued") {
        addEvent("JOB", `job=${short} → 큐 적재 (${r.pos}/${QUEUE_CAP})`);
      } else {
        addEvent(
          "SYSTEM",
          `job=${short} 거부 — QUEUE_FULL (슬롯·큐 ${QUEUE_CAP} 포화). [Monday 통지는 후속]`,
        );
      }
    })
      .then(track)
      .catch((e) => console.warn("[JobBridge] job:request listen 실패:", e));

    listenQueueClear(() => {
      const n = clearQueue();
      addEvent("SYSTEM", `큐 비우기 — ${n}건 제거 (실행중 슬롯 무관)`);
    })
      .then(track)
      .catch((e) => console.warn("[JobBridge] queue:clear listen 실패:", e));

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [addEvent, assignJob, clearQueue]);

  return null;
}
