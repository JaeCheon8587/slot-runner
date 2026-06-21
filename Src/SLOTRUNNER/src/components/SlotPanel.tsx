import { PtyTerminal } from "@/components/PtyTerminal";
import { stageDirective, STAGE_LABEL } from "@/lib/jobs";
import { useAppStore } from "@/store/useAppStore";

// 슬롯 패널: busy 면 대상 repo(cwd)에서 claude 세션 spawn + 스텝1(docs-add-task) 지시 주입.
// 스텝 2~3 전이는 StageController 가 Stop훅마다 수행. 완료/실패는 EndOfRunModal(F004/ADR-003).
// 완료(sim)/실패(sim) 버튼 = 수동 override(훅 미발화 등 대비).
const CLAUDE_ARGS: string[] = [];

export function SlotPanel({ id }: { id: string }) {
  const slot = useAppStore((s) => s.slots.find((x) => x.id === id));
  const setSlotOutcome = useAppStore((s) => s.setSlotOutcome);

  const job = slot?.status === "busy" ? slot.job : null;
  const pending = !!slot?.outcome;

  return (
    <div className="slot">
      <div className="slot-bar">
        <span className="slot-id">{id}</span>
        <span className={`badge badge-${slot?.status ?? "empty"}`}>
          {job ? `busy · ${STAGE_LABEL[slot!.stage]}` : "empty"}
        </span>
        {job && !pending ? (
          <>
            <button className="btn xs" onClick={() => setSlotOutcome(id, "done")}>
              완료(sim)
            </button>
            <button
              className="btn xs"
              onClick={() => setSlotOutcome(id, "failed", "sim 실패 사유")}
            >
              실패(sim)
            </button>
          </>
        ) : null}
      </div>
      {job ? (
        <PtyTerminal
          key={job.job_id}
          id={id}
          program="claude"
          args={CLAUDE_ARGS}
          cwd={job.cwd}
          inject={stageDirective(job, "docs-add-task")}
        />
      ) : (
        <div className="pty empty-pty">empty — 잡 대기</div>
      )}
    </div>
  );
}
