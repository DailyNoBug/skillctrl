import { invoke } from "@tauri-apps/api/core";
import type { CommandExecution } from "./types";

export async function locateSkillctrlBinary(): Promise<string> {
  return invoke<string>("locate_skillctrl_binary");
}

export async function runSkillctrl(args: string[]): Promise<CommandExecution> {
  return invoke<CommandExecution>("run_skillctrl", { args });
}
