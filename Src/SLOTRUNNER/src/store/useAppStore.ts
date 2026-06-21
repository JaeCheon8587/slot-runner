import { create } from "zustand";
import { effectiveStages, DONE, type Job } from "@/lib/jobs";

// 슬롯 풀 + FIFO 큐 (SLOTRUNNER-FRD-002 F002 / ADR-002). 기본 N=2, 큐 상한 10.
export const SLOT_IDS = ["slot-1", "slot-2"] as const;
export const QUEUE_CAP = 10;

export type ConsoleEvent = { id: string; ts: number; source: string; message: string };
export type SlotStatus = "empty" | "busy";
/** 파이프라인 결과 (슬라이스 6, F004/ADR-003). 실제 트리거는 forge/ddr 게이트, 현재는 sim. */
export type SlotOutcome = { kind: "done" | "failed"; reason?: string };
export type Slot = {
  id: string;
  status: SlotStatus;
  job: Job | null;
  /** 현재 스텝 인덱스(job.stages 기준, Supervisor 패턴 한 턴 한 단계). busy 슬롯에서만 의미. */
  stageIndex: number;
  /** 완료/실패 결과. null 이 아니면 EndOfRunModal 표시(사람 결정 게이트). */
  outcome: SlotOutcome | null;
};

export type AssignResult =
  | { kind: "assigned"; slotId: string }
  | { kind: "queued"; pos: number }
  | { kind: "rejected" };

export type ReleaseResult =
  | { kind: "reassigned"; slotId: string; job: Job }
  | { kind: "freed" };

type AppState = {
  consoleEvents: ConsoleEvent[];
  slots: Slot[];
  queue: Job[];
  /** 슬롯 claude 기동 후 forge 주입 지연(ms). config 로 갱신. */
  injectDelayMs: number;
  /** 스텝 전이 시 컨텍스트 점유율(%)이 이 값 이상이면 /compact 주입(StageController). */
  compactThresholdPct: number;
  /** 컨텍스트 점유율 분모(토큰). */
  contextWindowTokens: number;
  addEvent: (source: string, message: string) => void;
  clearConsole: () => void;
  /** 백엔드 config 적용 — 슬롯 수(빈 슬롯 재생성) + 주입 지연 + 컨텍스트 압축. 시작 시 1회. */
  applyConfig: (cfg: {
    slots: number;
    inject_delay_ms: number;
    compact_threshold_pct?: number;
    context_window_tokens?: number;
  }) => void;
  assignJob: (job: Job) => AssignResult;
  /** 슬롯 해제(세션 종료). 큐 대기분 있으면 즉시 재배정(dequeue), 없으면 빈 슬롯으로. outcome 초기화. */
  releaseSlot: (slotId: string) => ReleaseResult;
  /** 파이프라인 완료/실패 결과 설정 → EndOfRunModal 트리거(F004/ADR-003). */
  setSlotOutcome: (slotId: string, kind: SlotOutcome["kind"], reason?: string) => void;
  /** [유지] — outcome 만 해제, 슬롯은 busy 점유 유지(가용 슬롯 감소, ADR-002). */
  keepSession: (slotId: string) => void;
  /** 대기 큐 전체 비우기(실행중 슬롯 무관). 제거된 건수 반환. F002/AC-F002-004. */
  clearQueue: () => number;
  /** 스텝 루프 다음 단계로 전이. 다음 단계명 반환(끝나면 "done"). */
  advanceStage: (slotId: string) => string;
};

function newEvent(source: string, message: string): ConsoleEvent {
  return { id: crypto.randomUUID(), ts: Date.now(), source, message };
}

function emptySlot(id: string): Slot {
  return { id, status: "empty", job: null, stageIndex: 0, outcome: null };
}

export const useAppStore = create<AppState>((set, get) => ({
  consoleEvents: [
    newEvent("SYSTEM", `앱 시작 — REST :8765, 슬롯 ${SLOT_IDS.length}개, 큐 상한 ${QUEUE_CAP}`),
  ],
  slots: SLOT_IDS.map((id) => emptySlot(id)),
  queue: [],
  injectDelayMs: 4000,
  compactThresholdPct: 40,
  contextWindowTokens: 1_000_000,

  addEvent: (source, message) =>
    set((s) => ({ consoleEvents: [...s.consoleEvents, newEvent(source, message)] })),
  clearConsole: () => set({ consoleEvents: [] }),

  applyConfig: (cfg) => {
    const n = Math.max(1, Math.min(4, cfg.slots || 2));
    const ids = Array.from({ length: n }, (_, i) => `slot-${i + 1}`);
    set({
      slots: ids.map((id) => emptySlot(id)),
      injectDelayMs: cfg.inject_delay_ms || 4000,
      compactThresholdPct: Math.min(100, cfg.compact_threshold_pct ?? 40),
      contextWindowTokens: cfg.context_window_tokens || 1_000_000,
    });
  },

  assignJob: (job) => {
    const { slots, queue } = get();
    const idx = slots.findIndex((s) => s.status === "empty");
    if (idx >= 0) {
      const next = slots.slice();
      next[idx] = { ...next[idx], status: "busy", job, stageIndex: 0, outcome: null };
      set({ slots: next });
      return { kind: "assigned", slotId: next[idx].id };
    }
    if (queue.length < QUEUE_CAP) {
      set({ queue: [...queue, job] });
      return { kind: "queued", pos: queue.length + 1 };
    }
    return { kind: "rejected" };
  },

  releaseSlot: (slotId) => {
    const { slots, queue } = get();
    const idx = slots.findIndex((s) => s.id === slotId);
    if (idx < 0) return { kind: "freed" };
    if (queue.length > 0) {
      const [head, ...rest] = queue;
      const next = slots.slice();
      next[idx] = { ...next[idx], status: "busy", job: head, stageIndex: 0, outcome: null };
      set({ slots: next, queue: rest });
      return { kind: "reassigned", slotId, job: head };
    }
    const next = slots.slice();
    next[idx] = emptySlot(slotId);
    set({ slots: next });
    return { kind: "freed" };
  },

  setSlotOutcome: (slotId, kind, reason) =>
    set((s) => {
      const idx = s.slots.findIndex((x) => x.id === slotId);
      if (idx < 0 || s.slots[idx].status !== "busy") return s;
      const next = s.slots.slice();
      next[idx] = { ...next[idx], outcome: { kind, reason } };
      return { slots: next };
    }),

  keepSession: (slotId) =>
    set((s) => {
      const idx = s.slots.findIndex((x) => x.id === slotId);
      if (idx < 0) return s;
      const next = s.slots.slice();
      next[idx] = { ...next[idx], outcome: null };
      return { slots: next };
    }),

  clearQueue: () => {
    const n = get().queue.length;
    if (n > 0) set({ queue: [] });
    return n;
  },

  advanceStage: (slotId) => {
    const { slots } = get();
    const idx = slots.findIndex((s) => s.id === slotId);
    if (idx < 0 || !slots[idx].job) return DONE;
    const stages = effectiveStages(slots[idx].job as Job);
    const nextIndex = slots[idx].stageIndex + 1;
    const next = slots.slice();
    next[idx] = { ...next[idx], stageIndex: nextIndex };
    set({ slots: next });
    return stages[nextIndex] ?? DONE;
  },
}));
