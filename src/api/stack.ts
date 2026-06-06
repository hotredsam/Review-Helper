import { invoke } from "@tauri-apps/api/core";

export interface CatalogOption {
  choice: string;
  rationale: string;
}
export interface PremadeStack {
  name: string;
  summary: string;
  panes: Record<string, string>;
}
export interface Selection {
  pane: string;
  choice: string | null;
  alternatives: string | null;
  rationale: string | null;
}

export function stackCatalog(): Promise<Record<string, CatalogOption[]>> {
  return invoke<Record<string, CatalogOption[]>>("stack_catalog");
}
export function stackPremade(): Promise<PremadeStack[]> {
  return invoke<PremadeStack[]>("stack_premade");
}
export function stackList(id: number): Promise<Selection[]> {
  return invoke<Selection[]>("stack_list", { projectId: id });
}
export function stackSet(id: number, pane: string, choice: string): Promise<void> {
  return invoke("stack_set", { projectId: id, pane, choice });
}
export function stackApplyPremade(id: number, name: string): Promise<void> {
  return invoke("stack_apply_premade", { projectId: id, name });
}
