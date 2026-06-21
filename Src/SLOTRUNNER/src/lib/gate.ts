import { invoke } from "@tauri-apps/api/core";

// 단계 게이트 IPC (ADR-003). 백엔드 src-tauri/src/infra/gate.rs 와 대응.
export type StepStatus = { step: number; status: string };
export type ForgeGate = {
  /** ok | blocked | error | pending | interrupted | empty | missing | invalid */
  status: string;
  total: number;
  completed: number;
  steps: StepStatus[];
};

export const readForgeGate = (repo: string, phase: string) =>
  invoke<ForgeGate>("read_forge_gate", { repo, phase });

export const readDdrGate = (repo: string, phase: string, stem: string) =>
  invoke<boolean>("read_ddr_gate", { repo, phase, stem });
