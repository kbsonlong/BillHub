import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { BatchRecord, ImportPreview, LedgerEvent, LedgerSummary, PeriodSummary } from "../types";

export async function pickStatement(): Promise<string | null> {
  const selected = await open({
    title: "选择微信或支付宝账单",
    filters: [{ name: "账单文件", extensions: ["xlsx", "csv"] }],
    multiple: false,
    directory: false,
  });
  if (typeof selected === "string") return selected;
  if (selected && !Array.isArray(selected) && "path" in selected) {
    return String((selected as { path: string }).path);
  }
  return null;
}

export const previewStatement = (filePath: string) =>
  invoke<ImportPreview>("api_preview", { filePath });

export const importStatement = (filePath: string, replace: boolean) =>
  invoke<BatchRecord>("api_import", { filePath, replace });

export const loadBatches = () => invoke<BatchRecord[]>("api_batches");

export const loadEvents = (limit = 50, includeNeutral = false, includePending = false) =>
  invoke<LedgerEvent[]>("api_events", { limit, includeNeutral, includePending });

export const loadSummary = (includeNeutral = false, includePending = false) =>
  invoke<LedgerSummary>("api_summary", { includeNeutral, includePending });

export const loadMonthlySummary = () => invoke<PeriodSummary[]>("api_monthly_summary");

export const loadYearlySummary = () => invoke<PeriodSummary[]>("api_yearly_summary");

export const deleteBatch = (batchId: string) => invoke<boolean>("api_delete", { batchId });
