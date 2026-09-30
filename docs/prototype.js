(() => {
  const $ = (selector) => document.querySelector(selector);
  const views = [...document.querySelectorAll('.view')];
  const invoke = window.__TAURI__?.core?.invoke;
  const runtimeNotice = $('#runtime-notice');
  const feedback = $('#feedback');
  let selectedPath = null;
  let preview = null;
  let importBusy = false;

  const money = (cents) => new Intl.NumberFormat('zh-CN', {
    style: 'currency', currency: 'CNY', minimumFractionDigits: 2,
  }).format((Number(cents) || 0) / 100);
  const dateTime = (seconds) => {
    const date = new Date(Number(seconds) * 1000);
    return Number.isNaN(date.valueOf()) ? '时间未知' : new Intl.DateTimeFormat('zh-CN', {
      month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit',
    }).format(date);
  };
  const setText = (selector, value) => { $(selector).textContent = value; };
  const element = (tag, className, text) => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const formatError = (error) => {
    if (typeof error === 'string') {
      try {
        const parsed = JSON.parse(error);
        return parsed.message || parsed.code || error;
      } catch { return error; }
    }
    return error?.message || error?.code || '操作失败，请重试。';
  };
  const run = async (operation, action) => {
    try { return await action(); }
    catch (error) {
      feedback.textContent = `${operation}失败：${formatError(error)}`;
      feedback.classList.add('error');
      throw error;
    }
  };
  function showView(id) {
    const target = document.getElementById(id) || $('#today');
    views.forEach((view) => {
      const active = view === target;
      view.classList.toggle('active', active);
      view.setAttribute('aria-hidden', String(!active));
    });
    document.querySelectorAll('.nav-btn,.desktop-nav button').forEach((button) => {
      const active = button.dataset.go === target.id;
      button.classList.toggle('active', active);
      if (active) button.setAttribute('aria-current', 'page');
      else button.removeAttribute('aria-current');
    });
    $('.phone-content').scrollTo({
      top: 0,
      behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth',
    });
  }
  document.querySelectorAll('[data-go]').forEach((button) => button.addEventListener('click', () => showView(button.dataset.go)));
  showView('today');

  const now = new Date();
  const localMonth = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
  $('#period').value = localMonth;
  $('#period').addEventListener('change', () => refreshDashboard().catch(() => {}));

  function renderEvents(items) {
    const list = $('#ledger-list');
    list.replaceChildren();
    if (!items.length) {
      list.append(element('li', 'empty-state', '这个月还没有流水。'));
      return;
    }
    items.forEach((event) => {
      const row = element('li', 'ledger-row');
      const description = element('span', '', event.counterparty || event.description || event.raw_category || '未命名交易');
      const meta = element('small', '', `${dateTime(event.occurred_at)} · ${event.provider === 'wechat' ? '微信支付' : event.provider === 'alipay' ? '支付宝' : event.provider}`);
      description.append(meta);
      const amount = element('span', 'ledger-amount', `${event.cash_flow === 'income' || (event.event_kind === 'refund' && event.cash_flow === 'income') ? '+' : '−'}${money(event.amount_cents)}`);
      if (event.cash_flow === 'income') amount.classList.add('income');
      row.append(description, amount);
      list.append(row);
    });
  }
  function renderBatches(batches) {
    const list = $('#batch-list');
    list.replaceChildren();
    if (!batches.length) {
      list.append(element('li', 'empty-state', '还没有导入账单。'));
      return;
    }
    batches.slice(0, 5).forEach((batch) => {
      const row = element('li', 'batch-row');
      const label = element('span', '', batch.provider === 'wechat' ? '微信支付账单' : batch.provider === 'alipay' ? '支付宝账单' : batch.provider);
      label.append(element('small', '', `${dateTime(batch.imported_at)} · ${batch.accepted_count} 条已导入`));
      row.append(label, element('span', '', `${batch.rejected_count} 条未识别`));
      list.append(row);
    });
  }
  async function refreshDashboard() {
    if (!invoke) return;
    feedback.textContent = '';
    feedback.classList.remove('error');
    const period = $('#period').value || localMonth;
    setText('#today-period', `本地账本 · ${period}`);
    const [summary, month, events, batches] = await run('读取账单', () => Promise.all([
      invoke('api_summary', { includeNeutral: false, includePending: false }),
      invoke('api_month_dashboard', { period }),
      invoke('api_events', { page: 1, pageSize: 5, period }),
      invoke('api_batches'),
    ]));
    setText('#summary-expense', money(summary.settled_expense_cents));
    setText('#summary-income', money(summary.settled_income_cents));
    setText('#summary-pending', String(summary.pending_count));
    setText('#summary-unknown', String(summary.unknown_count));
    setText('#event-count', `共 ${events.total_count} 笔`);
    renderEvents(events.items || []);
    renderBatches(batches || []);
    setText('#month-income', money(month?.income_cents));
    setText('#month-expense', money(month?.expense_cents));
    setText('#month-net', money(month?.net_cents));
    const categories = $('#category-list');
    categories.replaceChildren();
    if (!month?.categories?.length) {
      categories.append(element('li', 'empty-state', '这个月暂无分类支出。'));
    } else {
      month.categories.forEach((category) => {
        const row = element('li', 'category-row');
        row.append(element('span', '', category.category), element('strong', '', money(category.amount_cents)));
        categories.append(row);
      });
    }
  }
  function resetPreview() {
    selectedPath = null;
    preview = null;
    setText('#selected-file', '尚未选择文件');
    $('#import-preview').hidden = true;
    $('#import-preview').replaceChildren();
    $('#confirm-import').hidden = true;
    $('#confirm-import').disabled = false;
  }
  function renderPreview(data) {
    const box = $('#import-preview');
    const summary = data.summary;
    box.replaceChildren();
    box.hidden = false;
    box.append(element('h2', '', '解析预览'));
    const stats = element('dl', 'preview-stats');
    [['识别记录', summary.record_count], ['可导入', summary.accepted_count], ['无法识别', summary.rejected_count]].forEach(([label, value]) => {
      stats.append(element('dt', '', label), element('dd', '', String(value)));
    });
    box.append(stats);
    if (data.existing_batch_id) {
      box.append(element('p', 'duplicate-warning', '这份文件已导入过。为避免重复记账，本次不能再次写入。'));
    }
    (summary.warnings || []).forEach((warning) => box.append(element('p', 'preview-warning', warning)));
    (summary.issues || []).slice(0, 5).forEach(([rowNumber, message]) => {
      box.append(element('p', 'preview-warning', `第 ${rowNumber} 行：${message}`));
    });
    $('#confirm-import').hidden = false;
    $('#confirm-import').disabled = Boolean(data.existing_batch_id) || summary.accepted_count === 0;
  }

  if (!invoke) {
    runtimeNotice.hidden = false;
    setText('#summary-expense', '桌面端可用');
    setText('#summary-income', '桌面端可用');
    setText('#summary-pending', '—');
    setText('#summary-unknown', '—');
    $('#ledger-list').replaceChildren(element('li', 'empty-state', '在 BillHub 桌面应用中查看本地账单。'));
    $('#batch-list').replaceChildren(element('li', 'empty-state', '在 BillHub 桌面应用中查看导入记录。'));
    $('#category-list').replaceChildren(element('li', 'empty-state', '在 BillHub 桌面应用中查看月度分类。'));
    $('#pick-statement').disabled = true;
    feedback.textContent = '当前页面不在 BillHub 桌面运行环境中，账单读取与导入不可用。';
  } else {
    refreshDashboard().catch(() => {});
  }

  $('#pick-statement').addEventListener('click', async () => {
    if (!invoke || importBusy) return;
    feedback.textContent = '';
    feedback.classList.remove('error');
    try {
      const path = await run('选择文件', () => invoke('api_pick_statement'));
      if (!path) return;
      resetPreview();
      selectedPath = path;
      setText('#selected-file', path.split(/[\\/]/).pop());
      preview = await run('账单解析', () => invoke('api_preview', { filePath: path }));
      renderPreview(preview);
    } catch { /* Error is shown in the status area. */ }
  });
  $('#confirm-import').addEventListener('click', async () => {
    if (!invoke || !selectedPath || !preview || preview.existing_batch_id || importBusy) return;
    importBusy = true;
    const button = $('#confirm-import');
    button.disabled = true;
    button.textContent = '正在导入…';
    feedback.textContent = '';
    feedback.classList.remove('error');
    try {
      const batch = await run('导入账单', () => invoke('api_import', { filePath: selectedPath, replace: false }));
      feedback.textContent = `导入完成：${batch.accepted_count} 条已写入本地账本。`;
      feedback.classList.remove('error');
      resetPreview();
      try {
        await refreshDashboard();
        feedback.textContent = `导入完成：${batch.accepted_count} 条已写入本地账本，账单数据已刷新。`;
      } catch {
        feedback.textContent = `导入已完成（${batch.accepted_count} 条），但账单刷新失败，请稍后重试。`;
        feedback.classList.add('error');
      }
    } catch { /* Error is shown in the status area. */ }
    finally {
      importBusy = false;
      button.textContent = '确认导入';
      button.disabled = Boolean(preview?.existing_batch_id) || !preview || preview.summary.accepted_count === 0;
    }
  });
})();
