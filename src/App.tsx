import { useCallback, useEffect, useRef, useState } from "react";
import type { BatchRecord, EventPage, ImportPreview, LedgerEvent, LedgerSummary, PeriodSummary } from "./types";
import {
  deleteBatch, importStatement, loadBatches, loadEvents, loadMonthlySummary, loadSummary, loadYearlySummary,
  pickStatement, previewStatement, updateEvent, updateEvents,
} from "./lib/api";
import { dateTime, fileName, labels, money } from "./lib/format";

type Feedback = { kind: "success" | "error"; text: string } | null;
type Tab = "import" | "events" | "analysis";
type AnalysisMode = "monthly" | "yearly";
type EventEditor = Pick<LedgerEvent, "id" | "event_kind" | "lifecycle">;

const eventKinds = ["payment", "refund", "transfer", "top_up", "withdrawal", "adjustment"];
const lifecycles = ["settled", "pending", "unknown", "closed", "reversed"];

function badgeClass(value: string) {
  return `badge badge-${value}`;
}

type TrendChartProps = {
  periods: PeriodSummary[];
  mode: AnalysisMode;
  onSelectPeriod: (period: string, category?: "income" | "expense") => void;
};

function chartMoney(amountCents: number): string {
  const sign = amountCents < 0 ? "-" : "";
  const amount = Math.abs(amountCents) / 100;
  if (amount >= 10_000) return `${sign}¥${(amount / 10_000).toFixed(1)}万`;
  if (amount >= 1_000) return `${sign}¥${(amount / 1_000).toFixed(1)}k`;
  return `${sign}¥${Math.round(amount).toLocaleString("zh-CN")}`;
}

function cashFlowMoney(amountCents: number, flow: "income" | "expense"): string {
  const signedAmount = flow === "expense" ? -amountCents : amountCents;
  return `${signedAmount > 0 ? "+" : ""}${money(signedAmount)}`;
}

function TrendChart({ periods, mode, onSelectPeriod }: TrendChartProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  if (!periods.length) return <div className="chart-empty">暂无可展示的趋势数据。</div>;

  const chartWidth = 760;
  const chartHeight = 300;
  const margin = { top: 20, right: 20, bottom: 42, left: 58 };
  const plotWidth = chartWidth - margin.left - margin.right;
  const plotHeight = chartHeight - margin.top - margin.bottom;
  const plottedValues = periods.flatMap((period) => [period.income_cents, period.expense_cents]);
  const maximum = Math.max(0, ...plottedValues);
  const minimum = Math.min(0, ...plottedValues);
  const valueRange = Math.max(1, maximum - minimum);
  const xPosition = (index: number) =>
    periods.length === 1
      ? margin.left + plotWidth / 2
      : margin.left + (index * plotWidth) / (periods.length - 1);
  const yPosition = (amountCents: number) =>
    margin.top + ((maximum - amountCents) / valueRange) * plotHeight;
  const baselineY = yPosition(0);
  const pointsFor = (field: "income_cents" | "expense_cents") =>
    periods.map((period, index) => ({
      x: xPosition(index),
      y: yPosition(period[field]),
    }));
  const pathFor = (points: { x: number; y: number }[]) =>
    points.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" ");
  const areaPathFor = (points: { x: number; y: number }[]) =>
    `${pathFor(points)} L ${points[points.length - 1].x} ${baselineY} L ${points[0].x} ${baselineY} Z`;
  const incomePoints = pointsFor("income_cents");
  const expensePoints = pointsFor("expense_cents");
  const linePath = (field: "income_cents" | "expense_cents") =>
    pathFor(field === "income_cents" ? incomePoints : expensePoints);
  const labelFor = (period: PeriodSummary) =>
    mode === "monthly" ? period.period.replace("-", "/") : period.period;
  const shouldShowLabel = (index: number) =>
    periods.length <= 8 || index === 0 || index === periods.length - 1 || index % 2 === 0;
  const latestPeriod = periods[periods.length - 1];
  const hoveredPeriod = hoveredIndex === null ? null : periods[hoveredIndex] ?? null;
  const hoveredX = hoveredPeriod && hoveredIndex !== null ? xPosition(hoveredIndex) : 0;
  const tooltipWidth = 188;
  const tooltipHeight = 54;
  const tooltipX = Math.min(
    Math.max(hoveredX - tooltipWidth / 2, margin.left),
    chartWidth - margin.right - tooltipWidth,
  );
  const tooltipY = hoveredPeriod
    ? Math.max(
        margin.top,
        Math.min(
          Math.min(yPosition(hoveredPeriod.income_cents), yPosition(hoveredPeriod.expense_cents)) - tooltipHeight - 8,
          margin.top + plotHeight - tooltipHeight,
        ),
      )
    : 0;
  const yTicks = [0, 0.25, 0.5, 0.75, 1].map(
    (ratio) => minimum + valueRange * ratio,
  );

  return (
    <svg className="trend-chart" viewBox={`0 0 ${chartWidth} ${chartHeight}`} role="group" aria-label={`${mode === "monthly" ? "月度" : "年度"}收入支出趋势图，可点击周期查看流水`} onMouseLeave={() => setHoveredIndex(null)}>
      <defs>
        <linearGradient id="income-area-gradient" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="var(--green)" stopOpacity=".22" />
          <stop offset="100%" stopColor="var(--green)" stopOpacity="0" />
        </linearGradient>
        <linearGradient id="expense-area-gradient" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="var(--red)" stopOpacity=".18" />
          <stop offset="100%" stopColor="var(--red)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect className="chart-plot-background" x={margin.left} y={margin.top} width={plotWidth} height={plotHeight} rx="10" />
      {yTicks.map((tickValue) => {
        const y = yPosition(tickValue);
        return (
          <g key={tickValue}>
            <line className="chart-gridline" x1={margin.left} x2={chartWidth - margin.right} y1={y} y2={y} />
            <text className="chart-axis-label" x={margin.left - 10} y={y + 4} textAnchor="end">{chartMoney(tickValue)}</text>
          </g>
        );
      })}
      <path className="chart-income-area" d={areaPathFor(incomePoints)} />
      <path className="chart-expense-area" d={areaPathFor(expensePoints)} />
      <path className="chart-income" d={linePath("income_cents")} />
      <path className="chart-expense" d={linePath("expense_cents")} />
      {periods.map((period, index) => (
        <g key={period.period}>
          <title>{`${period.period} · 收入 ${money(period.income_cents)} · 支出 ${money(period.expense_cents)}`}</title>
          <rect className="chart-hit-area" x={xPosition(index) - Math.max(18, plotWidth / periods.length / 2)} y={margin.top} width={Math.max(36, plotWidth / periods.length)} height={plotHeight} role="button" tabIndex={0} aria-label={`查看 ${period.period} 的流水`} onMouseEnter={() => setHoveredIndex(index)} onClick={() => onSelectPeriod(period.period)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelectPeriod(period.period); } }} />
          <circle className="chart-income-point chart-category-point" cx={xPosition(index)} cy={yPosition(period.income_cents)} r="4" role="button" tabIndex={0} aria-label={`查看 ${period.period} 的收入流水`} onClick={() => onSelectPeriod(period.period, "income")} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelectPeriod(period.period, "income"); } }} />
          <circle className="chart-expense-point chart-category-point" cx={xPosition(index)} cy={yPosition(period.expense_cents)} r="4" role="button" tabIndex={0} aria-label={`查看 ${period.period} 的支出流水`} onClick={() => onSelectPeriod(period.period, "expense")} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onSelectPeriod(period.period, "expense"); } }} />
          {shouldShowLabel(index) && <text className="chart-period-label" x={xPosition(index)} y={chartHeight - 14} textAnchor="middle">{labelFor(period)}</text>}
        </g>
      ))}
      {hoveredIndex !== null && hoveredPeriod && (
        <g className="chart-tooltip" pointerEvents="none">
          <line className="chart-hover-line" x1={hoveredX} x2={hoveredX} y1={margin.top} y2={baselineY} />
          <rect className="chart-tooltip-background" x={tooltipX} y={tooltipY} width={tooltipWidth} height={tooltipHeight} rx="8" />
          <text className="chart-tooltip-period" x={tooltipX + 12} y={tooltipY + 18}>{hoveredPeriod.period}</text>
          <text className="chart-tooltip-value income-tooltip" x={tooltipX + 12} y={tooltipY + 38}>收入 {money(hoveredPeriod.income_cents)}</text>
          <text className="chart-tooltip-value expense-tooltip" x={tooltipX + 104} y={tooltipY + 38}>支出 {money(hoveredPeriod.expense_cents)}</text>
        </g>
      )}
      <text className="chart-latest-label" x={chartWidth - margin.right} y={margin.top - 4} textAnchor="end">最新 {latestPeriod.period}</text>
    </svg>
  );
}

export default function App() {
  const [tab, setTab] = useState<Tab>("import");
  const [filePath, setFilePath] = useState("");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [replace, setReplace] = useState(false);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<Feedback>(null);
  const [batches, setBatches] = useState<BatchRecord[]>([]);
  const [events, setEvents] = useState<LedgerEvent[]>([]);
  const [eventPage, setEventPage] = useState<EventPage | null>(null);
  const [eventsLoading, setEventsLoading] = useState(false);
  const [summary, setSummary] = useState<LedgerSummary | null>(null);
  const [monthly, setMonthly] = useState<PeriodSummary[]>([]);
  const [yearly, setYearly] = useState<PeriodSummary[]>([]);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(50);
  const [providerFilter, setProviderFilter] = useState("all");
  const [cashFlowFilter, setCashFlowFilter] = useState("all");
  const [lifecycleFilter, setLifecycleFilter] = useState("all");
  const [periodFilter, setPeriodFilter] = useState<string | null>(null);
  const [analysisMode, setAnalysisMode] = useState<AnalysisMode>("monthly");
  const [editingEvent, setEditingEvent] = useState<EventEditor | null>(null);
  const [selectedEventIds, setSelectedEventIds] = useState<string[]>([]);
  const [batchEventKind, setBatchEventKind] = useState("");
  const [batchLifecycle, setBatchLifecycle] = useState("");
  const eventRequestId = useRef(0);

  const refresh = useCallback(async () => {
    try {
      const [nextBatches, nextSummary, nextMonthly, nextYearly] = await Promise.all([
        loadBatches(),
        loadSummary(false, false),
        loadMonthlySummary(),
        loadYearlySummary(),
      ]);
      setBatches(nextBatches);
      setSummary(nextSummary);
      setMonthly(nextMonthly);
      setYearly(nextYearly);
    } catch (error) {
      setFeedback({ kind: "error", text: String(error) });
    }
  }, []);

  const refreshEventPage = useCallback(async () => {
    const requestId = ++eventRequestId.current;
    setEventsLoading(true);
    try {
      const nextEventPage = await loadEvents({
        page,
        pageSize,
        provider: providerFilter === "all" ? undefined : providerFilter,
        cashFlow: cashFlowFilter === "all" ? undefined : cashFlowFilter,
        lifecycle: lifecycleFilter === "all" ? undefined : lifecycleFilter,
        period: periodFilter ?? undefined,
      });
      if (requestId !== eventRequestId.current) return;
      setEvents(nextEventPage.items);
      setEventPage(nextEventPage);
    } catch (error) {
      if (requestId === eventRequestId.current) {
        setFeedback({ kind: "error", text: String(error) });
      }
    } finally {
      if (requestId === eventRequestId.current) setEventsLoading(false);
    }
  }, [page, pageSize, providerFilter, cashFlowFilter, lifecycleFilter, periodFilter]);

  useEffect(() => { void refresh(); }, [refresh]);
  useEffect(() => { void refreshEventPage(); }, [refreshEventPage]);

  const chooseFile = async () => {
    setFeedback(null);
    setBusy(true);
    try {
      const selected = await pickStatement();
      if (!selected) return;
      setFilePath(selected);
      setPreview(null);
      setPreview(await previewStatement(selected));
    }
    catch (error) { setFeedback({ kind: "error", text: String(error) }); }
    finally { setBusy(false); }
  };

  const importFile = async () => {
    if (!filePath) return;
    setBusy(true); setFeedback(null);
    try {
      const batch = await importStatement(filePath, replace);
      setFeedback({ kind: "success", text: `导入成功：${labels[batch.provider] ?? batch.provider} ${batch.accepted_count} 条` });
      setPreview(null); setFilePath(""); await refresh(); await refreshEventPage(); setTab("events");
    } catch (error) { setFeedback({ kind: "error", text: String(error) }); }
    finally { setBusy(false); }
  };

  const removeBatch = async (batch: BatchRecord) => {
    if (!window.confirm(`撤销批次将删除 ${batch.accepted_count} 条已入账记录，是否继续？`)) return;
    try { await deleteBatch(batch.id); await refresh(); await refreshEventPage(); }
    catch (error) { setFeedback({ kind: "error", text: String(error) }); }
  };

  const saveEvent = async () => {
    if (!editingEvent) return;
    setBusy(true); setFeedback(null);
    try {
      const updated = await updateEvent(editingEvent.id, editingEvent.event_kind, editingEvent.lifecycle);
      if (!updated) throw new Error("未找到要修改的流水，请刷新后重试。");
      setEditingEvent(null);
      setFeedback({ kind: "success", text: "流水类型和状态已更新。" });
      await refresh();
      await refreshEventPage();
    } catch (error) { setFeedback({ kind: "error", text: String(error) }); }
    finally { setBusy(false); }
  };

  const toggleEventSelection = (eventId: string, selected: boolean) => {
    setSelectedEventIds((current) => selected
      ? current.includes(eventId) ? current : [...current, eventId]
      : current.filter((id) => id !== eventId));
  };

  const toggleVisibleSelection = (selected: boolean) => {
    setSelectedEventIds((current) => {
      const next = new Set(current);
      events.forEach((event) => selected ? next.add(event.id) : next.delete(event.id));
      return [...next];
    });
  };

  const saveBatchEvents = async () => {
    if (!selectedEventIds.length || (!batchEventKind && !batchLifecycle)) return;
    setBusy(true); setFeedback(null);
    try {
      const updated = await updateEvents(
        selectedEventIds,
        batchEventKind || undefined,
        batchLifecycle || undefined,
      );
      setSelectedEventIds([]);
      setBatchEventKind("");
      setBatchLifecycle("");
      setFeedback({ kind: "success", text: `已批量更新 ${updated} 条流水。` });
      await refresh();
      await refreshEventPage();
    } catch (error) { setFeedback({ kind: "error", text: String(error) }); }
    finally { setBusy(false); }
  };

  const allVisibleSelected = events.length > 0 && events.every((event) => selectedEventIds.includes(event.id));

  const showPeriodEvents = (period: string, category?: "income" | "expense") => {
    setPeriodFilter(period);
    setCashFlowFilter(category ?? "all");
    setPage(1);
    setTab("events");
  };

  const periods = analysisMode === "monthly" ? monthly : yearly;
  const analysisTotals = periods.reduce(
    (totals, period) => ({
      income: totals.income + period.income_cents,
      expense: totals.expense + period.expense_cents,
      net: totals.net + period.net_cents,
      count: totals.count + period.transaction_count,
    }),
    { income: 0, expense: 0, net: 0, count: 0 },
  );

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand"><span>BillHub</span><small>本地账单导入</small></div>
        <nav>
          <button className={tab === "import" ? "active" : ""} onClick={() => setTab("import")}>导入中心</button>
          <button className={tab === "events" ? "active" : ""} onClick={() => setTab("events")}>流水</button>
          <button className={tab === "analysis" ? "active" : ""} onClick={() => setTab("analysis")}>收支分析</button>
        </nav>
        <div className="privacy">本地优先 · 不上传账单</div>
      </aside>

      <main className="content">
        <header className="page-head">
          <div>
            <h1>{tab === "import" ? "导入中心" : tab === "events" ? "统一流水" : "收支分析"}</h1>
            <p>{tab === "import" ? "预览后再入账，完整批次可撤销" : tab === "events" ? "默认展示全部流水，可按平台、收支和状态筛选并编辑确认结果" : "实际支出已扣除退款金额"}</p>
          </div>
          {summary && (
            <div className="summary-strip">
              <span>实际支出<strong>{money(summary.settled_expense_cents - summary.refund_income_cents)}</strong></span>
              <span>收入<strong>{money(summary.settled_income_cents - summary.refund_expense_cents)}</strong></span>
              <span>待确认<strong>{summary.pending_count}</strong></span>
              <span>待核实<strong>{summary.unknown_count}</strong></span>
              <span>中性<strong>{summary.neutral_count}</strong></span>
            </div>
          )}
        </header>

        {feedback && <div className={`feedback ${feedback.kind}`}>{feedback.text}</div>}

        {tab === "import" && (
          <section className="import-grid">
            <div className="drop-card">
              <div className="drop-icon">↧</div>
              <h2>选择账单文件</h2>
              <p>支持微信 XLSX 和支付宝 CSV，解析在本地完成。</p>
              <button className="primary" onClick={chooseFile} disabled={busy}>{busy ? "解析中..." : "选择文件"}</button>
              {filePath && <div className="selected-file" title={filePath}>{fileName(filePath)}</div>}
            </div>

            <div className="preview-card">
              <h2>导入预览</h2>
              {!preview && <p className="empty">请选择文件生成字段、记录数和错误行预览。</p>}
              {preview && (
                <>
                  <div className="preview-grid">
                    <div><small>识别平台</small><strong>{labels[preview.summary.parser_id.split("_")[0]] ?? preview.summary.parser_id}</strong></div>
                    <div><small>记录数</small><strong>{preview.summary.record_count}</strong></div>
                    <div><small>可入账</small><strong>{preview.summary.accepted_count}</strong></div>
                    <div><small>错误</small><strong>{preview.summary.rejected_count}</strong></div>
                  </div>
                  <dl><dt>时间范围</dt><dd>{dateTime(preview.summary.range_start)} 至 {dateTime(preview.summary.range_end)}</dd></dl>
                  <dl><dt>SHA256</dt><dd className="mono">{preview.file_sha256}</dd></dl>
                  <label className="replace">
                    <input type="checkbox" checked={replace} onChange={(e) => setReplace(e.target.checked)} />
                    替换同指纹批次
                  </label>
                  <div className="issues">
                    {preview.summary.warnings.map((warning) => <div key={warning} className="issue warning">{warning}</div>)}
                    {preview.summary.issues.map(([row, message]) => (
                      <div key={`${row}-${message}`} className="issue">行 {row}: {message}</div>
                    ))}
                  </div>
                  <div className="preview-actions">
                    <button className="primary" onClick={importFile} disabled={busy || preview.summary.accepted_count === 0}>确认导入</button>
                    <button onClick={() => { setPreview(null); setFilePath(""); }}>取消</button>
                  </div>
                </>
              )}
            </div>

            <div className="batch-card">
              <h2>导入批次</h2>
              {!batches.length && <p className="empty">暂无批次。</p>}
              {batches.map((batch) => (
                <article key={batch.id}>
                  <div>
                    <strong>{labels[batch.provider] ?? batch.provider}</strong>
                    <span>{dateTime(batch.imported_at)}</span>
                  </div>
                  <p>{batch.accepted_count} 条入账 · {batch.rejected_count} 条问题</p>
                  <div className="batch-actions">
                    <code>{batch.file_sha256.slice(0, 12)}</code>
                    <button onClick={() => removeBatch(batch)}>撤销</button>
                  </div>
                </article>
              ))}
            </div>
          </section>
        )}

        {tab === "events" && (
          <section className="events-panel">
            <div className="filters">
              <label>平台<select value={providerFilter} onChange={(e) => { setProviderFilter(e.target.value); setPage(1); }}><option value="all">全部</option><option value="wechat">微信</option><option value="alipay">支付宝</option></select></label>
              <label>分类<select value={cashFlowFilter} onChange={(e) => { setCashFlowFilter(e.target.value); setPage(1); }}><option value="all">全部</option><option value="expense">支出</option><option value="income">收入</option><option value="refund">退款</option><option value="neutral">中性</option></select></label>
              <span className="refund-note">退款不计入收入</span>
              <label>状态<select value={lifecycleFilter} onChange={(e) => { setLifecycleFilter(e.target.value); setPage(1); }}><option value="all">全部</option><option value="settled">已完成</option><option value="pending">待确认</option><option value="unknown">待核实</option><option value="closed">已关闭</option><option value="reversed">已反转</option></select></label>
              {periodFilter && <span className="period-filter">时间：{periodFilter}<button onClick={() => { setPeriodFilter(null); setPage(1); }}>清除</button></span>}
              <span>共 {eventPage?.total_count ?? 0} 条</span>
            </div>
            {selectedEventIds.length > 0 && (
              <div className="batch-editor">
                <strong>已选 {selectedEventIds.length} 条</strong>
                <label>类型<select value={batchEventKind} onChange={(e) => setBatchEventKind(e.target.value)}><option value="">不修改</option>{eventKinds.map((kind) => <option key={kind} value={kind}>{labels[kind] ?? kind}</option>)}</select></label>
                <label>状态<select value={batchLifecycle} onChange={(e) => setBatchLifecycle(e.target.value)}><option value="">不修改</option>{lifecycles.map((lifecycle) => <option key={lifecycle} value={lifecycle}>{labels[lifecycle] ?? lifecycle}</option>)}</select></label>
                <button className="primary" onClick={saveBatchEvents} disabled={busy || (!batchEventKind && !batchLifecycle)}>批量保存</button>
                <button onClick={() => setSelectedEventIds([])} disabled={busy}>取消选择</button>
              </div>
            )}
            <div className="table-wrap" aria-busy={eventsLoading}>
              <table>
                <thead><tr><th><input aria-label="全选当前页" type="checkbox" checked={allVisibleSelected} onChange={(e) => toggleVisibleSelection(e.target.checked)} /></th><th>时间</th><th>平台</th><th>类型</th><th>对方/说明</th><th>账户</th><th>状态</th><th>金额</th><th>操作</th></tr></thead>
                <tbody>
                  {events.map((event) => (
                    <tr key={event.id}>
                      <td><input aria-label={`选择 ${event.provider_transaction_id}`} type="checkbox" checked={selectedEventIds.includes(event.id)} onChange={(e) => toggleEventSelection(event.id, e.target.checked)} /></td>
                      <td>{dateTime(event.occurred_at)}</td>
                      <td>{labels[event.provider] ?? event.provider}</td>
                      <td>{labels[event.event_kind] ?? event.event_kind}</td>
                      <td><strong>{event.counterparty ?? "—"}</strong><small>{event.description ?? ""}</small></td>
                      <td>{event.funding_account ?? "—"}</td>
                      <td><span className={badgeClass(event.lifecycle)}>{labels[event.lifecycle] ?? event.lifecycle}</span></td>
                      <td className={`amount ${event.cash_flow}`}>{event.cash_flow === "expense" ? "-" : event.cash_flow === "income" ? "+" : ""}{money(event.amount_cents)}</td>
                      <td className="event-actions">
                        {editingEvent?.id === event.id ? (
                          <div className="event-editor">
                            <select aria-label="交易类型" value={editingEvent.event_kind} onChange={(e) => setEditingEvent({ ...editingEvent, event_kind: e.target.value })}>
                              {eventKinds.map((kind) => <option key={kind} value={kind}>{labels[kind] ?? kind}</option>)}
                            </select>
                            <select aria-label="交易状态" value={editingEvent.lifecycle} onChange={(e) => setEditingEvent({ ...editingEvent, lifecycle: e.target.value })}>
                              {lifecycles.map((lifecycle) => <option key={lifecycle} value={lifecycle}>{labels[lifecycle] ?? lifecycle}</option>)}
                            </select>
                            <button className="primary" onClick={saveEvent} disabled={busy}>保存</button>
                            <button onClick={() => setEditingEvent(null)} disabled={busy}>取消</button>
                          </div>
                        ) : <button onClick={() => setEditingEvent({ id: event.id, event_kind: event.event_kind, lifecycle: event.lifecycle })} disabled={busy}>编辑</button>}
                      </td>
                    </tr>
                  ))}
                  {!events.length && <tr><td colSpan={9} className="empty">{eventsLoading ? "正在加载流水…" : "暂无流水。"}</td></tr>}
                </tbody>
              </table>
            </div>
            {eventPage && eventPage.total_pages > 0 && (
              <div className="pagination">
                <label>每页<select value={pageSize} onChange={(e) => { setPageSize(Number(e.target.value)); setPage(1); }}><option value={20}>20</option><option value={50}>50</option><option value={100}>100</option></select> 条</label>
                <button onClick={() => setPage((current) => current - 1)} disabled={eventPage.page <= 1}>上一页</button>
                <span>第 {eventPage.page} / {eventPage.total_pages} 页</span>
                <button onClick={() => setPage((current) => current + 1)} disabled={eventPage.page >= eventPage.total_pages}>下一页</button>
              </div>
            )}
          </section>
        )}

        {tab === "analysis" && (
          <section className="analysis-panel">
            <div className="analysis-toolbar">
              <div className="segmented-control">
                <button className={analysisMode === "monthly" ? "selected" : ""} onClick={() => setAnalysisMode("monthly")}>按月</button>
                <button className={analysisMode === "yearly" ? "selected" : ""} onClick={() => setAnalysisMode("yearly")}>按年</button>
              </div>
            </div>

            <div className="analysis-cards">
              <article><small>累计收入</small><strong className="income-text">{money(analysisTotals.income)}</strong></article>
              <article><small>累计实际支出</small><strong className="expense-text">{money(analysisTotals.expense)}</strong></article>
              <article><small>累计结余</small><strong className={analysisTotals.net >= 0 ? "income-text" : "expense-text"}>{money(analysisTotals.net)}</strong></article>
              <article><small>{analysisMode === "monthly" ? "月份" : "年份"} / 笔数</small><strong>{periods.length} / {analysisTotals.count}</strong></article>
            </div>

            <div className="trend-card">
              <div className="trend-header">
                <div><h2>收支趋势</h2><p>点击周期查看全部，点击收入或支出数据查看对应分类</p></div>
                <div className="trend-legend">
                  <span><i className="legend-dot income-dot" />收入</span>
                  <span><i className="legend-dot expense-dot" />实际支出</span>
                </div>
              </div>
              {periods.length > 0 && <div className="trend-latest"><span>最新周期</span><strong>{periods[periods.length - 1].period}</strong><em className="income-text">收入 {money(periods[periods.length - 1].income_cents)}</em><em className="expense-text">实际支出 {money(periods[periods.length - 1].expense_cents)}</em></div>}
              <TrendChart periods={periods} mode={analysisMode} onSelectPeriod={showPeriodEvents} />
            </div>

            <div className="table-wrap analysis-table">
              <table>
                <thead><tr><th>{analysisMode === "monthly" ? "月份" : "年份"}</th><th>收入</th><th>实际支出</th><th>净额</th><th>交易笔数</th></tr></thead>
                <tbody>
                  {periods.map((period) => (
                    <tr className="period-row" key={period.period}>
                      <td><button className="period-link" onClick={() => showPeriodEvents(period.period)}>{period.period}</button></td>
                      <td className="amount income-text"><button className="period-amount-button" onClick={() => showPeriodEvents(period.period, "income")}>{cashFlowMoney(period.income_cents, "income")}</button></td>
                      <td className="amount expense-text"><button className="period-amount-button" onClick={() => showPeriodEvents(period.period, "expense")}>{cashFlowMoney(period.expense_cents, "expense")}</button></td>
                      <td className={`amount ${period.net_cents >= 0 ? "income-text" : "expense-text"}`}>{money(period.net_cents)}</td>
                      <td>{period.transaction_count}</td>
                    </tr>
                  ))}
                  {!periods.length && <tr><td colSpan={5} className="empty">暂无可分析的收支记录，请先导入账单。</td></tr>}
                </tbody>
              </table>
            </div>
          </section>
        )}
      </main>
    </div>
  );
}
