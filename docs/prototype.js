    const views = [...document.querySelectorAll('.view')];
    function showView(id) {
      const target = document.getElementById(id) || document.getElementById('today');
      views.forEach(view => { const active = view === target; view.classList.toggle('active', active); view.setAttribute('aria-hidden', String(!active)); });
      document.querySelectorAll('.nav-btn,.desktop-nav button').forEach(button => { const active = button.dataset.go === target.id; button.classList.toggle('active', active); if (active) button.setAttribute('aria-current','page'); else button.removeAttribute('aria-current'); });
      document.querySelector('.phone-content').scrollTo({top:0,behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'auto':'smooth'});
    }
    document.querySelectorAll('[data-go]').forEach(button => button.addEventListener('click', () => showView(button.dataset.go)));
    document.querySelectorAll('.quest').forEach(button => button.addEventListener('click', () => { document.getElementById('quest-name').textContent = button.dataset.quest; showView('detail'); }));
    showView('today');
    const entryForm = document.getElementById('entry-form');
    const amountInput = document.getElementById('amount');
    const categoryInput = document.getElementById('category');
    function validateField(field) {
      const error = document.getElementById(`${field.id}-error`);
      const invalid = field.id === 'amount' ? !(Number(field.value) > 0) : !field.value;
      field.setAttribute('aria-invalid', String(invalid));
      error.textContent = invalid ? (field.id === 'amount' ? '请输入大于 0 的金额。' : '请选择一个账目分类。') : '';
      return !invalid;
    }
    [amountInput, categoryInput].forEach(field => field.addEventListener('blur', () => validateField(field)));
    entryForm.addEventListener('submit', event => {
      event.preventDefault();
      const amountValid = validateField(amountInput);
      const categoryValid = validateField(categoryInput);
      if (!amountValid || !categoryValid) { (amountValid ? categoryInput : amountInput).focus(); return; }
      const button = document.getElementById('start-quest'); button.disabled = true; button.textContent = '任务已完成 ✓';
      document.getElementById('feedback').textContent = `已记录 ¥${Number(amountInput.value).toFixed(2)} · ${categoryInput.value}。经验值 +30 XP（演示数据）`;
      const selectedQuest = document.querySelector(`.quest[data-quest="${CSS.escape(document.getElementById('quest-name').textContent)}"]`);
      if (selectedQuest) selectedQuest.classList.add('completed');
      const row = document.createElement('li'); row.className = 'ledger-row';
      const description = document.createElement('span'); description.textContent = `${document.getElementById('quest-name').textContent} · ${categoryInput.value}`;
      const meta = document.createElement('small'); meta.textContent = '刚刚 · 本次演示'; description.appendChild(meta);
      const value = document.createElement('span'); value.textContent = `¥${Number(amountInput.value).toFixed(2)}`;
      row.append(description, value); document.getElementById('ledger-list').prepend(row);
      const progress = document.querySelector('.progress i'); progress.style.width = '68%';
      document.querySelector('.progress').setAttribute('aria-valuenow','68');
      document.querySelector('.xp-totals').textContent = '1,678 / 2,480 XP';
      document.querySelector('.xp-foot span').textContent = '距离下一级还差 802 XP';
    });
