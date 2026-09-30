export type PreviewSummary = {
  parser_id: string;
  parser_version: string;
  record_count: number;
  accepted_count: number;
  rejected_count: number;
  range_start: number | null;
  range_end: number | null;
  warnings: string[];
  issues: [number, string][];
};

export type ImportPreview = {
  summary: PreviewSummary;
  file_sha256: string;
  existing_batch_id: string | null;
};

export type BatchRecord = {
  id: string;
  provider: string;
  file_sha256: string;
  record_count: number;
  accepted_count: number;
  rejected_count: number;
  imported_at: number;
};

export type LedgerEvent = {
  id: string;
  batch_id: string | null;
  provider: string;
  occurred_at: number;
  event_kind: string;
  cash_flow: string;
  lifecycle: string;
  amount_cents: number;
  counterparty: string | null;
  description: string | null;
  raw_category: string | null;
  funding_account: string | null;
  provider_transaction_id: string | null;
  merchant_order_id: string | null;
  raw_json: string;
};

export type DailyTask = { id: string; title: string; description: string; xp: number; completed: boolean };
export type GamificationSnapshot = {
  total_xp: number;
  level: number;
  xp_into_level: number;
  next_level_xp: number;
  streak_days: number;
  week: { date: string; xp: number; completed: boolean }[];
  tasks: DailyTask[];
};

export type EventPage = {
  items: LedgerEvent[];
  total_count: number;
  page: number;
  page_size: number;
  total_pages: number;
};

export type EventFilters = {
  page: number;
  pageSize: number;
  provider?: string;
  cashFlow?: string;
  lifecycle?: string;
  period?: string;
};

export type LedgerSummary = {
  settled_expense_cents: number;
  settled_income_cents: number;
  refund_income_cents: number;
  refund_expense_cents: number;
  pending_count: number;
  unknown_count: number;
  neutral_count: number;
};

export type PeriodSummary = {
  period: string;
  income_cents: number;
  expense_cents: number;
  net_cents: number;
  transaction_count: number;
};

export type MonthDashboard = {
  period: string;
  income_cents: number;
  expense_cents: number;
  net_cents: number;
  transaction_count: number;
  pending_count: number;
  categories: { category: string; amount_cents: number }[];
  daily_expenses: { day: number; amount_cents: number }[];
  recent_events: LedgerEvent[];
};
