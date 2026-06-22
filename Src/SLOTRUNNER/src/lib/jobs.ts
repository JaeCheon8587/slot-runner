import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// Rust domain::job::Job 와 1:1. 코드상세 SSOT = src-tauri/src/domain/job.rs.
export type Job = {
  job_id: string;
  /** 프로젝트 논리명(봇이 보냄). Rust 가 레지스트리로 cwd/sln/app 해석 후 emit — 프론트엔 해석 완료본 도착. */
  project?: string | null;
  /** 대상 repo 경로(슬롯 claude 세션 cwd). project 로 해석되거나 직접 지정. */
  cwd: string;
  /** 대상 App 코드 (docs-add-task 입력, 예: MASTER). */
  app: string;
  /** forge 워크트리 격리 슬러그. */
  phase: string;
  sln: string;
  test_target?: string | null;
  /** 자연어 요구사항(docs-add-task 입력). */
  prompt: string;
  /** 입력 문서 — A(설계)=요구사항 .md / B(개발)=TASK .md. 비면 prompt 만 사용. */
  doc?: string | null;
  /** 잡별 routine(스텝 순서). 비면 DEFAULT_STAGES. 봇이 Monday 키워드로 채움(설계→full / 개발→forge+ddr). */
  stages?: string[] | null;
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

// ── 스텝 루프 routine (Supervisor 패턴, 잡별 가변) ─────────────────────────
// 한 턴 한 단계. StageController 가 Stop훅마다 다음 단계 지시를 주입한다.
// 단계는 문자열(스킬명). job.stages 가 비면 DEFAULT_STAGES.
export const DEFAULT_STAGES = ["docs-add-task", "forge-scope", "ddr-loop"] as const;
export const DONE = "done";
// Monday 통지 종결 스텝 — 봇이 보낸 작업 stages 뒤에 SlotRunner 가 자동 추가(ADR-010).
// 파이프라인 종점=Monday(F003). update_id 있을 때만(통지 대상 존재).
export const MONDAY_STAGE = "monday-notify";
// Monday 답글 제목 끝 시그니처(요구). 제목은 항상 이 문자열로 끝난다.
export const MONDAY_SIGNATURE = "- Claude Code Agent with Codex Agent";
// Monday 답글에 멘션할 대상(요구). mentionsList 로 전달해야 실제 알림이 간다(본문 '@이름'은 plain text).
// type: User | Team | Board | Project. 대상 추가 시 이 배열에 행 추가.
export const MONDAY_MENTIONS: { id: string; type: string; name: string }[] = [
  { id: "105405262", type: "User", name: "Brenda" }, // Brenda (Integration Agent)
  { id: "940702", type: "Team", name: "ROS팀" },
];
const MONDAY_MENTIONS_JSON = JSON.stringify(
  MONDAY_MENTIONS.map((m) => ({ id: m.id, type: m.type })),
);
const MONDAY_MENTION_NAMES = MONDAY_MENTIONS.map((m) => m.name).join(", ");

/// 이 잡의 실효 routine. stages 지정 시 그대로(비면 기본 풀 routine) + Monday 종결 스텝 자동 추가.
/// 설계·개발 어느 routine이든 마지막은 monday-notify(중복 방지, update_id 있을 때).
export function effectiveStages(job: Job): string[] {
  const base = job.stages && job.stages.length > 0 ? [...job.stages] : [...DEFAULT_STAGES];
  if (job.update_id && base[base.length - 1] !== MONDAY_STAGE) {
    base.push(MONDAY_STAGE);
  }
  return base;
}

// 플러그인 스킬 네임스페이스 — bare 이름은 "Unknown skill". 반드시 plugin:skill.
const PLUGIN = "claudecode-for-me";

const HEADLESS_RULES =
  "규칙: AskUserQuestion·사용자 확인 금지(무인), run_in_background/Monitor 금지(이 세션이 끝까지 블로킹). " +
  "이 단계 하나만 수행하고 끝나면 '턴을 종료'하라(다음 단계는 외부 감독이 지시한다). " +
  `스킬은 반드시 '${PLUGIN}:<스킬명>' 네임스페이스로 호출(bare 는 Unknown skill).`;

function head(stage: string, idx: number, total: number): string {
  return `[스텝 ${idx + 1}/${total} · ${stage}] ${HEADLESS_RULES}`;
}

// 단계명 → 지시 빌더. 알려진 파이프라인 스킬은 전용 지시, 그 외는 제너릭 호출.
const STAGE_DIRECTIVES: Record<string, (job: Job) => string> = {
  "docs-add-task": (job) => {
    const prompt = (job.prompt ?? "").replace(/\s+/g, " ").trim() || job.phase;
    const docRef = job.doc ? `\n[참고 요구사항 문서] ${job.doc}` : "";
    return `\`${PLUGIN}:docs-add-task\` 스킬로 아래 요구사항의 ${job.app} App 설계문서(TASK 등)를 작성하라(codex 자동 채점 수렴까지).\n[요구사항] ${prompt}${docRef}`;
  },
  "forge-scope": (job) => {
    const tt = job.test_target ? ` test-target=${job.test_target}` : "";
    // A(docs-add-task 선행)는 방금 만든 TASK, B(forge부터)는 잡의 doc 을 대상으로.
    const target = job.doc
      ? `직전 단계에서 TASK 를 만들었으면 그 TASK 를, 아니면 본 잡 문서 ${job.doc} 를`
      : "직전 단계에서 생성한 TASK 문서를";
    return `${target} 대상으로 \`${PLUGIN}:forge-scope\` 스킬을 --single-step 으로 실행해 구현+빌드+테스트하라.\n[phase] ${job.phase}   [빌드] sln=${job.sln}${tt}`;
  },
  "ddr-loop": (job) =>
    `직전 forge-scope 구현(워크트리 브랜치 feat-${job.phase})을 \`${PLUGIN}:ddr-loop\` 스킬로 문서 기준 검증·수렴하라.`,
  // 종결 스텝 — 작업 결과를 Monday update 답글로 통지(파이프라인 종점, ADR-010). Monday MCP 사용.
  "monday-notify": (job) =>
    `방금 수행한 작업(phase ${job.phase})의 결과를 요약해 Monday MCP \`create_update\` 로 답글을 작성하라.\n` +
    `- board_id: ${job.board_id}\n- item_id(pulse): ${job.item_id}\n` +
    `- 답글 대상 update_id: ${job.update_id} (parentId 로 지정해 reply 로 등록)\n` +
    `- 본문: 무엇을(엔드포인트/기능) 구현·검증했는지 + 빌드/테스트 결과를 간결히. 본문은 글자 그대로 등록.\n` +
    `- 본문 첫 줄(제목)은 반드시 시그니처 "${MONDAY_SIGNATURE}" 로 끝나게 하라(예: "[<TASK-ID>] <요약> ${MONDAY_SIGNATURE}").\n` +
    `- create_update 호출 시 mentionsList 를 반드시 전달하라(이것이 ${MONDAY_MENTION_NAMES} 에게 실제 멘션·알림을 발생시킨다): ` +
    `mentionsList=${MONDAY_MENTIONS_JSON}.\n` +
    `- 본문(body)에는 '@' 로 멘션을 적지 마라(plain text 로만 남음). 멘션은 오직 mentionsList 로 처리한다.`,
};

/// 슬롯 claude 세션에 주입할 단계 지시. stage="done"/미정 → null.
export function stageDirective(job: Job, stage: string): string | null {
  if (!stage || stage === DONE) return null;
  const stages = effectiveStages(job);
  const idx = stages.indexOf(stage);
  const builder = STAGE_DIRECTIVES[stage];
  const body = builder
    ? builder(job)
    : `\`${PLUGIN}:${stage}\` 스킬을 실행하라.`; // 미지 스킬 제너릭
  return `${head(stage, idx < 0 ? 0 : idx, stages.length)}\n${body}`;
}

const KNOWN_LABEL: Record<string, string> = {
  "docs-add-task": "docs-add-task",
  "forge-scope": "forge-scope",
  "ddr-loop": "ddr-loop",
  "monday-notify": "Monday 통지",
  done: "완료",
};
export function stageLabel(stage: string): string {
  return KNOWN_LABEL[stage] ?? stage;
}
