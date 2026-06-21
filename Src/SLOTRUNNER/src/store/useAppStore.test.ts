import { beforeEach, describe, expect, it } from "vitest";
import { useAppStore, QUEUE_CAP, SLOT_IDS } from "./useAppStore";
import type { Job } from "@/lib/jobs";

function mkJob(n: number): Job {
  return {
    job_id: `id-${n}`,
    cwd: "C:/repo",
    app: "MASTER",
    phase: `p${n}`,
    sln: "s",
    prompt: "x",
    board_id: "1",
    item_id: "2",
    update_id: "3",
  };
}

const st = () => useAppStore.getState();

beforeEach(() => {
  useAppStore.setState({
    slots: SLOT_IDS.map((id) => ({
      id,
      status: "empty" as const,
      job: null,
      stageIndex: 0,
      outcome: null,
    })),
    queue: [],
    consoleEvents: [],
  });
});

describe("슬롯 풀 + FIFO 큐 (F002 / ADR-002)", () => {
  it("빈 슬롯에 순서대로 배정, 가득 차면 큐 적재", () => {
    expect(st().assignJob(mkJob(1))).toEqual({ kind: "assigned", slotId: "slot-1" });
    expect(st().assignJob(mkJob(2))).toEqual({ kind: "assigned", slotId: "slot-2" });
    const r = st().assignJob(mkJob(3));
    expect(r).toEqual({ kind: "queued", pos: 1 });
    expect(st().queue.length).toBe(1);
  });

  it("큐 상한 10 초과 시 거부", () => {
    st().assignJob(mkJob(1));
    st().assignJob(mkJob(2)); // 슬롯 2개 busy
    for (let i = 0; i < QUEUE_CAP; i++) st().assignJob(mkJob(100 + i)); // 큐 10 채움
    expect(st().queue.length).toBe(QUEUE_CAP);
    expect(st().assignJob(mkJob(999))).toEqual({ kind: "rejected" });
    expect(st().queue.length).toBe(QUEUE_CAP); // 거부분 미적재
  });

  it("슬롯 해제 시 큐 대기분 즉시 재배정(dequeue, FIFO)", () => {
    st().assignJob(mkJob(1)); // slot-1
    st().assignJob(mkJob(2)); // slot-2
    st().assignJob(mkJob(3)); // 큐
    const r = st().releaseSlot("slot-1");
    expect(r).toMatchObject({ kind: "reassigned", slotId: "slot-1" });
    expect(st().queue.length).toBe(0);
    expect(st().slots.find((x) => x.id === "slot-1")?.job?.job_id).toBe("id-3");
  });

  it("큐 비었을 때 해제는 빈 슬롯으로", () => {
    st().assignJob(mkJob(1));
    expect(st().releaseSlot("slot-1")).toEqual({ kind: "freed" });
    expect(st().slots.find((x) => x.id === "slot-1")?.status).toBe("empty");
  });
});

describe("큐 비우기 / config (F002 AC-F002-004 / SPEC §5)", () => {
  it("clearQueue 는 대기 큐만 비우고 제거 건수 반환", () => {
    st().assignJob(mkJob(1));
    st().assignJob(mkJob(2)); // 슬롯 2 busy
    st().assignJob(mkJob(3));
    st().assignJob(mkJob(4)); // 큐 2건
    expect(st().queue.length).toBe(2);
    expect(st().clearQueue()).toBe(2);
    expect(st().queue.length).toBe(0);
    // 실행중 슬롯은 무관
    expect(st().slots.filter((s) => s.status === "busy").length).toBe(2);
  });

  it("clearQueue 빈 큐면 0", () => {
    expect(st().clearQueue()).toBe(0);
  });

  it("applyConfig 슬롯 수 적용(1..4 clamp) + 주입지연", () => {
    st().applyConfig({ slots: 3, inject_delay_ms: 1500 });
    expect(st().slots.length).toBe(3);
    expect(st().slots.map((s) => s.id)).toEqual(["slot-1", "slot-2", "slot-3"]);
    expect(st().injectDelayMs).toBe(1500);
    st().applyConfig({ slots: 9, inject_delay_ms: 0 });
    expect(st().slots.length).toBe(4); // clamp
  });
});

describe("스텝 루프 단계 전이 (Supervisor 패턴)", () => {
  it("배정 시 docs-add-task 부터, advanceStage 로 순차 전이 + Monday 종결 후 done", () => {
    st().assignJob(mkJob(1)); // update_id 있음 → monday-notify 종결 자동 추가
    expect(st().slots.find((s) => s.id === "slot-1")?.stageIndex).toBe(0);
    expect(st().advanceStage("slot-1")).toBe("forge-scope");
    expect(st().advanceStage("slot-1")).toBe("ddr-loop");
    expect(st().advanceStage("slot-1")).toBe("monday-notify"); // 종결 스텝
    expect(st().advanceStage("slot-1")).toBe("done");
    expect(st().advanceStage("slot-1")).toBe("done"); // done 에서 더 안 넘어감
  });

  it("잡별 stages 따름 (B: forge-scope→ddr-loop) + Monday 종결", () => {
    st().assignJob({ ...mkJob(1), stages: ["forge-scope", "ddr-loop"] });
    expect(st().slots.find((s) => s.id === "slot-1")?.stageIndex).toBe(0);
    expect(st().advanceStage("slot-1")).toBe("ddr-loop");
    expect(st().advanceStage("slot-1")).toBe("monday-notify"); // 종결 스텝
    expect(st().advanceStage("slot-1")).toBe("done");
  });

  it("재배정 시 단계가 docs-add-task 로 초기화", () => {
    st().assignJob(mkJob(1));
    st().assignJob(mkJob(2));
    st().assignJob(mkJob(3)); // 큐
    st().advanceStage("slot-1"); // forge-scope
    st().releaseSlot("slot-1"); // job3 재배정
    expect(st().slots.find((s) => s.id === "slot-1")?.stageIndex).toBe(0);
  });
});

describe("완료/유지 결정 게이트 (F004 / ADR-003)", () => {
  it("setSlotOutcome 은 busy 슬롯에만 outcome 설정", () => {
    st().assignJob(mkJob(1)); // slot-1 busy
    st().setSlotOutcome("slot-1", "done");
    expect(st().slots.find((x) => x.id === "slot-1")?.outcome).toEqual({
      kind: "done",
      reason: undefined,
    });
    // empty 슬롯엔 무효
    st().setSlotOutcome("slot-2", "failed", "x");
    expect(st().slots.find((x) => x.id === "slot-2")?.outcome).toBeNull();
  });

  it("[유지]는 outcome 만 해제, busy 점유 유지", () => {
    st().assignJob(mkJob(1));
    st().setSlotOutcome("slot-1", "done");
    st().keepSession("slot-1");
    const s1 = st().slots.find((x) => x.id === "slot-1");
    expect(s1?.outcome).toBeNull();
    expect(s1?.status).toBe("busy");
    expect(s1?.job?.job_id).toBe("id-1");
  });

  it("[종료]=releaseSlot 은 outcome 초기화 + 슬롯 해제(또는 dequeue)", () => {
    st().assignJob(mkJob(1)); // slot-1
    st().assignJob(mkJob(2)); // slot-2
    st().assignJob(mkJob(3)); // 큐
    st().setSlotOutcome("slot-1", "failed", "r");
    const r = st().releaseSlot("slot-1");
    expect(r).toMatchObject({ kind: "reassigned" });
    const s1 = st().slots.find((x) => x.id === "slot-1");
    expect(s1?.outcome).toBeNull(); // 재배정 시 outcome 초기화
    expect(s1?.job?.job_id).toBe("id-3");
  });
});
