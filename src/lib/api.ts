import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { BatchRecord, EventFilters, EventPage, GamificationSnapshot, ImportPreview, LedgerEvent, LedgerSummary, MonthDashboard, PeriodSummary } from "../types";

const DESKTOP_REQUIRED = "当前为浏览器预览模式，请使用 npm run dev 启动 BillHub 桌面端。";

function ensureDesktop(): void {
  if (!isTauri()) throw new Error(DESKTOP_REQUIRED);
}

function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  ensureDesktop();
  return invoke<T>(command, args);
}

export async function pickStatement(): Promise<string | null> {
  ensureDesktop();
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
  invokeDesktop<ImportPreview>("api_preview", { filePath });

export const importStatement = (filePath: string, replace: boolean, taskDay: string) =>
  invokeDesktop<BatchRecord>("api_import", { filePath, replace, taskDay });

export const loadGamification = (day: string) =>
  invokeDesktop<GamificationSnapshot>("api_gamification", { day });

export const completeDailyTask = (taskId: string, day: string) =>
  invokeDesktop<GamificationSnapshot>("api_complete_daily_task", { taskId, day });

export const createManualEntry = (entry: { occurredAt: number; amountCents: number; cashFlow: string; category: string; description: string; taskDay: string }) =>
  invokeDesktop<LedgerEvent>("api_create_manual_entry", entry);

export const loadBatches = () => invokeDesktop<BatchRecord[]>("api_batches");

export const loadEvents = ({ page, pageSize, provider, cashFlow, lifecycle, period }: EventFilters) =>
  invokeDesktop<EventPage>("api_events", { page, pageSize, provider, cashFlow, lifecycle, period });

export const updateEvent = (eventId: string, eventKind: string, lifecycle: string) =>
  invokeDesktop<boolean>("api_update_event", { eventId, eventKind, lifecycle });

export const updateEvents = (eventIds: string[], eventKind?: string, lifecycle?: string) =>
  invokeDesktop<number>("api_update_events", { eventIds, eventKind, lifecycle });

export const loadSummary = (includeNeutral = false, includePending = false) =>
  invokeDesktop<LedgerSummary>("api_summary", { includeNeutral, includePending });

export const loadMonthlySummary = () => invokeDesktop<PeriodSummary[]>("api_monthly_summary");

export const loadYearlySummary = () => invokeDesktop<PeriodSummary[]>("api_yearly_summary");

export const loadMonthDashboard = (period: string) =>
  invokeDesktop<MonthDashboard | null>("api_month_dashboard", { period });

export const deleteBatch = (batchId: string) => invokeDesktop<boolean>("api_delete", { batchId });
