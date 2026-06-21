import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// Rust domain::job::Job 와 1:1. 코드상세 SSOT = src-tauri/src/domain/job.rs.
export type Job = {
  job_id: string;
  /** 대상 repo 경로(슬롯 claude 세션 cwd). */
  cwd: string;
  /** 대상 App 코드 (docs-add-task 입력, 예: MASTER). */
  app: string;
  /** forge 워크트리 격리 슬러그. */
  phase: string;
  sln: string;
  test_target?: string | null;
  /** 자연어 요구사항(docs-add-task 입력). */
  prompt: string;
  board_id: string;
  item_id: string;
  update_id: string;
};

/// 백엔드 REST 인테이크가 emit 하는 "job:request" 를 구독한다.
export function listenJobRequest(cb: (job: Job) => void): Promise<UnlistenFn> {
  return listen<Job>("job:request", (e) => cb(e.payload));
}

/// REST `POST /jobs/queue:clear` → 백엔드가 emit 하는 큐 비우기 요청 구독.
export function listenQueueClear(cb: () => void): Promise<UnlistenFn> {
  return listen("queue:clear", () => cb());
}

// 스텝 루프(Supervisor 패턴): 한 턴 한 단계. 외부(StageController)가 Stop훅마다 다음 단계 지시 주입.
export type Stage = "docs-add-task" | "forge-scope" | "ddr-loop" | "done";

// 플러그인 스킬 네임스페이스 — bare 이름(docs-add-task)은 레지스트리에 없어 "Unknown skill" 난다.
// 반드시 plugin:skill 형식으로 호출한다.
const PLUGIN = "claudecode-for-me";

const HEADLESS_RULES =
  "규칙: AskUserQuestion·사용자 확인 금지(무인), run_in_background/Monitor 금지(이 세션이 끝까지 블로킹). " +
  "이 단계 하나만 수행하고 끝나면 '턴을 종료'하라(다음 단계는 외부 감독이 지시한다). " +
  `스킬은 반드시 네임스페이스 포함 '${PLUGIN}:<스킬명>' 으로 호출하라(bare 이름은 Unknown skill 오류).`;

/// 슬롯 claude 세션에 주입할 단계별 지시. 단계마다 1턴. 스킬은 plugin:skill 형식.
export function stageDirective(job: Job, stage: Stage): string | null {
  const prompt = (job.prompt ?? "").replace(/\s+/g, " ").trim() || job.phase;
  const tt = job.test_target ? ` test-target=${job.test_target}` : "";
  switch (stage) {
    case "docs-add-task":
      return [
        `[스텝 1/3 · docs-add-task] ${HEADLESS_RULES}`,
        `\`${PLUGIN}:docs-add-task\` 스킬로 아래 요구사항의 ${job.app} App 설계문서(TASK 등)를 작성하라(codex 자동 채점 수렴까지).`,
        `[요구사항] ${prompt}`,
      ].join("\n");
    case "forge-scope":
      return [
        `[스텝 2/3 · forge-scope] ${HEADLESS_RULES}`,
        `직전 단계에서 생성한 TASK 문서를 대상으로 \`${PLUGIN}:forge-scope\` 스킬을 --single-step 으로 실행해 구현+빌드+테스트하라.`,
        `[phase] ${job.phase}   [빌드] sln=${job.sln}${tt}`,
      ].join("\n");
    case "ddr-loop":
      return [
        `[스텝 3/3 · ddr-loop] ${HEADLESS_RULES}`,
        `직전 forge-scope 구현(워크트리 브랜치 feat-${job.phase})을 \`${PLUGIN}:ddr-loop\` 스킬로 문서 기준 검증·수렴하라.`,
      ].join("\n");
    case "done":
      return null;
  }
}

export const STAGE_LABEL: Record<Stage, string> = {
  "docs-add-task": "docs-add-task",
  "forge-scope": "forge-scope",
  "ddr-loop": "ddr-loop",
  done: "완료",
};
