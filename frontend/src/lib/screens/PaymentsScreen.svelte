<script lang="ts">
  import PaymentCalendarPanel from "../components/PaymentCalendarPanel.svelte";
  import SkeletonCard from "../components/SkeletonCard.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import { paymentsStore } from "../stores/payments";
  import type { PaymentDraftFormDto, PaymentMatchCandidateDto } from "../types";

  const payments = paymentsStore;
  let importButton: HTMLButtonElement | null = null;

  function onPaymentFieldChange(field: keyof PaymentDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;
    payments.updateFormField(field, input.value);
  }

  function onManualSearchInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    payments.updateManualMatchQuery(input.value);
  }

  function onSplitAllocationInput(documentId: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    payments.updateSplitAllocationAmount(documentId, input.value);
  }

  function focusImportButton() {
    importButton?.focus();
  }

  function getPaymentStateLabel(matchedDoc: string): string {
    return matchedDoc ? `Зв'язано з ${matchedDoc}` : "Не зведено";
  }

  function getDocumentKindLabel(kind: PaymentMatchCandidateDto["documentKind"]): string {
    return kind === "act" ? "Акт" : "Накладна";
  }

  function getPreviewTitle(): string {
    const preview = $payments.matchPreview;
    if (!preview) {
      return "";
    }

    if (preview.decisionKind === "exact") {
      return "Рекомендована звірка";
    }

    if (preview.decisionKind === "ambiguous") {
      return "Кілька кандидатів на звірку";
    }

    if (preview.decisionKind === "split") {
      return "Рекомендований розподіл платежу";
    }

    return "Автоматична звірка не знайшла точного документа";
  }

  function getPreviewDescription(): string {
    const preview = $payments.matchPreview;
    if (!preview) {
      return "";
    }

    if (preview.decisionKind === "exact") {
      return "Система знайшла найкращий документ для автозіставлення. Перевірте рекомендацію перед підтвердженням.";
    }

    if (preview.decisionKind === "ambiguous") {
      return "Оберіть найкращий варіант у списку, або відкрийте ручний пошук, якщо потрібен інший документ.";
    }

    if (preview.decisionKind === "split") {
      return "Система підготувала рекомендований розподіл платежу між кількома документами. Перевірте кандидатів і, за потреби, скоригуйте суми в чернетці нижче.";
    }

    return "Для цього платежу поки немає точного збігу. Перевірте реквізити або відкрийте ручний пошук документа.";
  }

  function getCandidateHint(candidate: PaymentMatchCandidateDto): string {
    const hints: string[] = [];

    if (candidate.sameIban) {
      hints.push("той самий IBAN");
    }

    if (candidate.referenceHit) {
      hints.push("є збіг по призначенню");
    }

    if (candidate.textHits > 0) {
      hints.push(`текстових збігів: ${candidate.textHits}`);
    }

    hints.push(`відхилення по даті: ${candidate.daysDistance} дн.`);
    return hints.join(" • ");
  }

  function runHeaderReconciliation() {
    const target = unmatchedPayments[0];
    if (target) {
      payments.reconcile(target.id);
    }
  }

  function isPaymentBusy(paymentId: string): boolean {
    return $payments.loading && $payments.activePaymentId === paymentId;
  }

  function openManualPickerForCurrentPreview() {
    const paymentId = $payments.matchPreview?.paymentId;
    if (paymentId) {
      payments.openManualMatchPicker(paymentId);
    }
  }

  function getFlowTitle(): string | null {
    if ($payments.loading && $payments.activeAction === "import") {
      return "Імпорт триває";
    }

    if ($payments.loading && $payments.activeAction === "import-pick") {
      return "Готуємо preview виписки";
    }

    if ($payments.loading && $payments.activeAction === "import-commit") {
      return "Імпортуємо нові платежі";
    }

    if ($payments.loading && $payments.activeAction === "sync") {
      return "Оновлюємо рухи з банку";
    }

    if ($payments.loading && $payments.activeAction === "reconcile") {
      return "Готуємо preview звірки";
    }

    if ($payments.loading && $payments.activeAction === "manual-search") {
      return "Шукаємо документи для ручної звірки";
    }

    if ($payments.loading && $payments.activeAction === "unreconcile") {
      return "Знімаємо зведення";
    }

    if ($payments.loading && $payments.activeAction === "save") {
      return "Зберігаємо платіж";
    }

    if ($payments.loading && $payments.activeAction === "confirm-auto-match") {
      return "Підтверджуємо автозвірку";
    }

    if ($payments.loading && $payments.activeAction === "confirm-candidate") {
      return "Підтверджуємо ручну звірку";
    }

    if ($payments.loading && $payments.activeAction === "confirm-manual-picker") {
      return "Фіксуємо ручний вибір документа";
    }

    if ($payments.loading && $payments.activeAction === "confirm-split") {
      return "Зберігаємо розподіл платежу";
    }

    return null;
  }

  function getFlowDescription(): string | null {
    if ($payments.loading && $payments.activeAction === "import") {
      return "Імпортуємо виписку та оновлюємо список платежів, щоб одразу показати незведені рухи.";
    }

    if ($payments.loading && $payments.activeAction === "import-pick") {
      return "Розбираємо файл виписки і будуємо список платежів, що чекають на імпорт.";
    }

    if ($payments.loading && $payments.activeAction === "import-commit") {
      return "Записуємо нові платежі у БД на основі підтвердженого preview.";
    }

    if ($payments.loading && $payments.activeAction === "sync") {
      return "Підтягуємо свіжі банківські рухи та готуємо їх до наступного кроку звірки.";
    }

    if ($payments.loading && $payments.activeAction === "reconcile") {
      return "Шукаємо документи-кандидати й готуємо наступний крок для цього платежу.";
    }

    if ($payments.loading && $payments.activeAction === "manual-search") {
      return "Формуємо повний список відкритих актів і накладних для ручного вибору.";
    }

    if ($payments.loading && $payments.activeAction === "unreconcile") {
      return "Знімаємо зв'язок із документом та повертаємо платіж у чергу на повторну звірку.";
    }

    if ($payments.loading && $payments.activeAction === "save") {
      return "Фіксуємо зміни в картці платежу та оновлюємо список.";
    }

    if ($payments.loading && $payments.activeAction === "confirm-auto-match") {
      return "Підтверджуємо рекомендоване автозіставлення і оновлюємо статус платежу.";
    }

    if ($payments.loading && $payments.activeAction === "confirm-candidate") {
      return "Прив'язуємо платіж до вибраного кандидата з preview.";
    }

    if ($payments.loading && $payments.activeAction === "confirm-manual-picker") {
      return "Прив'язуємо платіж до документа, обраного через ручний пошук.";
    }

    if ($payments.loading && $payments.activeAction === "confirm-split") {
      return "Записуємо розподіл платежу між кількома документами і оновлюємо статуси.";
    }

    return null;
  }

  $: items = $payments.list?.items ?? [];
  $: unmatchedPayments = items.filter((item) => !item.matchedDoc);
  $: matchedPayments = items.filter((item) => Boolean(item.matchedDoc));
  $: busyImport = $payments.loading && $payments.activeAction === "import";
  $: busyImportPick = $payments.loading && $payments.activeAction === "import-pick";
  $: busyImportCommit = $payments.loading && $payments.activeAction === "import-commit";
  $: busySync = $payments.loading && $payments.activeAction === "sync";
  $: busySave = $payments.loading && $payments.activeAction === "save";
  $: manualPickerCanConfirm = Boolean(
    $payments.manualPicker?.selectedCandidateId && ($payments.manualPicker?.candidates.length ?? 0) > 0
  );
  $: manualPickerDisabledReason =
    !$payments.manualPicker || $payments.manualPicker.candidates.length > 0
      ? ""
      : "Спершу знайдіть хоча б одного кандидата, щоб підтвердити документ.";
  $: flowTitle = getFlowTitle();
  $: flowDescription = getFlowDescription();
</script>

<section class="panel" data-testid="payments-screen">
  <div class="panel-header">
    <div>
      <h2>Платежі</h2>
      <p>{$payments.list?.items.length ?? 0} записів</p>
    </div>
  </div>

  <div class="create-strip-card">
    <div class="create-strip-header">
      <div>
        <strong>Контроль руху грошей</strong>
        <p>Працюємо від імпорту до звірки, а ручний платіж додаємо лише коли його справді не вистачає.</p>
      </div>
      <span class="doc-kind-badge">Звірка в центрі уваги</span>
    </div>

    <div class="payment-actions-grid">
      <article class="payment-action-card payment-action-card-primary">
        <div class="payment-action-copy">
          <span>Імпорт</span>
          <strong>Завантажте виписку і відразу побачте нові рухи</strong>
          <p>CSV-імпорт лишається головним входом у сценарій, а допоміжні дії не відволікають від нього.</p>
        </div>
        <div class="payment-action-buttons">
          <button
            bind:this={importButton}
            class="btn-primary"
            on:click={() => payments.pickAndPreviewImport()}
            disabled={busyImportPick || busyImport || busyImportCommit}
          >
            {busyImportPick ? "Готуємо preview..." : "Імпортувати виписку"}
          </button>
          <button
            class="btn-ghost"
            on:click={() => payments.importCsv()}
            disabled={busyImport || busyImportPick || busyImportCommit}
          >
            {busyImport ? "Імпортуємо виписку з storage..." : "Імпорт з storage/import/bank"}
          </button>
          <button class="btn-ghost" on:click={() => payments.syncBank()} disabled={busyImport || busySync || busyImportPick}>
            {busySync ? "Оновлюємо з банку..." : "Оновити з банку"}
          </button>
          <button class="btn-ghost" on:click={() => payments.openManualTemplate()} disabled={busyImport || busySync}>
            Шаблон CSV
          </button>
        </div>
      </article>

      <article class="payment-action-card">
        <div class="payment-action-copy">
          <span>Звірка</span>
          <strong>Почніть із незведених платежів і проведіть їх по одному</strong>
          <p>{unmatchedPayments.length} платежів чекають на зв'язок із документом або повторну перевірку реквізитів.</p>
        </div>
        <div class="payment-action-buttons">
          <button
            class="btn-secondary"
            on:click={runHeaderReconciliation}
            disabled={unmatchedPayments.length === 0 || $payments.loading}
          >
            Запустити звірку
          </button>
        </div>
      </article>

      <article class="payment-action-card">
        <div class="payment-action-copy">
          <span>Ручний платіж</span>
          <strong>Додавайте винятки окремо від потоку імпорту</strong>
          <p>Картка платежу вирівняна під звірку: напрям, сума, контрагент, референс і зв'язок із документом.</p>
        </div>
        <div class="payment-action-buttons">
          <button class="btn-secondary" on:click={() => payments.openEditor()} disabled={$payments.loading}>
            Створити платіж
          </button>
        </div>
      </article>
    </div>
  </div>

  <div class="task-kpis" data-testid="payments-kpis">
    {#if $payments.initialLoading}
      <SkeletonCard count={4} />
    {:else}
      <div class="task-kpi-card">
        <strong>{$payments.list?.kpi.incomingStr ?? "0,00"}</strong>
        <span>{$payments.list?.kpi.incomingSub ?? "надходження"}</span>
      </div>
      <div class="task-kpi-card">
        <strong>{$payments.list?.kpi.outgoingStr ?? "0,00"}</strong>
        <span>{$payments.list?.kpi.outgoingSub ?? "витрати"}</span>
      </div>
      <div class="task-kpi-card">
        <strong>{$payments.list?.kpi.netStr ?? "0,00"}</strong>
        <span>Баланс</span>
      </div>
      <div class="task-kpi-card task-kpi-card-alert">
        <strong>{$payments.list?.kpi.unmatchedCount ?? 0}</strong>
        <span>Не зведено</span>
      </div>
    {/if}
  </div>

  <PaymentCalendarPanel />

  {#if flowTitle}
    <section class="flow-banner status-banner is-loading" data-testid="payments-flow-banner" role="status" aria-live="polite">
      <strong>{flowTitle}</strong>
      {#if flowDescription}
        <p>{flowDescription}</p>
      {/if}
    </section>
  {/if}

  {#if $payments.message && !$payments.loading}
    <div class="status-banner is-success" role="status" aria-live="polite">
      <div>
        <strong>Дію виконано</strong>
        <p>{$payments.message}</p>
      </div>
    </div>
  {/if}

  {#if $payments.error}
    <div class="status-banner is-error" role="alert" aria-live="assertive">
      <div>
        <strong>Потрібна увага</strong>
        <p>{$payments.error}</p>
      </div>
    </div>
  {/if}

  {#if $payments.importPreview}
    <section class="chain-panel" data-testid="payments-import-preview" aria-label="Попередній перегляд імпорту виписки">
      <div class="chain-panel-header">
        <div>
          <strong>Попередній перегляд: {$payments.importPreview.bankName || "виписка"}</strong>
          <p>{$payments.importPreview.message}</p>
          <p class="payment-import-preview-path"><code>{$payments.importPreview.path}</code></p>
        </div>
        <div class="editor-actions">
          <button
            class="btn-primary"
            on:click={() => payments.commitImportPreview()}
            disabled={busyImportCommit || $payments.importPreview.willCreate === 0}
          >
            {busyImportCommit
              ? "Імпортуємо..."
              : $payments.importPreview.willCreate === 0
              ? "Немає нових платежів"
              : `Імпортувати ${$payments.importPreview.willCreate} платежі(ів)`}
          </button>
          <button
            class="btn-ghost"
            on:click={() => payments.cancelImportPreview()}
            disabled={busyImportCommit}
          >
            Скасувати
          </button>
        </div>
      </div>

      <ul class="payment-import-preview-summary">
        <li><span>Розпізнано рядків</span><strong>{$payments.importPreview.parsed}</strong></li>
        <li><span>Буде створено</span><strong>{$payments.importPreview.willCreate}</strong></li>
        <li><span>Уже існує (skip)</span><strong>{$payments.importPreview.willSkip}</strong></li>
        {#if $payments.importPreview.conflicts > 0}
          <li class="payment-import-preview-conflict"><span>Конфлікти</span><strong>{$payments.importPreview.conflicts}</strong></li>
        {/if}
      </ul>

      {#if $payments.importPreview.rows.length > 0}
        <table class="payment-import-preview-table">
          <thead>
            <tr>
              <th scope="col">Дія</th>
              <th scope="col">Bank ref</th>
              <th scope="col">Призначення</th>
              <th scope="col">Нотатка</th>
            </tr>
          </thead>
          <tbody>
            {#each $payments.importPreview.rows.slice(0, 25) as row, idx (idx)}
              <tr class:is-skipped={row.action === "skip"}>
                <td>{row.action === "create" ? "Нове" : "Пропуск"}</td>
                <td>{row.bankRef || "—"}</td>
                <td>{row.description || "—"}</td>
                <td>{row.note}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if $payments.importPreview.rows.length > 25}
          <p class="payment-import-preview-more">
            Показано перші 25 з {$payments.importPreview.rows.length} рядків.
          </p>
        {/if}
      {:else}
        <p class="payment-import-preview-empty">У файлі не знайдено жодного рядка виписки.</p>
      {/if}
    </section>
  {/if}

  {#if $payments.matchPreview}
    <section class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>{getPreviewTitle()}</strong>
          <p>{getPreviewDescription()}</p>
        </div>
        <div class="editor-actions">
          {#if $payments.matchPreview.decisionKind === "exact" && $payments.matchPreview.autoMatch}
            <button
              class="btn-primary"
              on:click={() => payments.confirmPreviewAutoMatch()}
              disabled={$payments.loading}
            >
              Підтвердити автозіставлення
            </button>
          {:else if $payments.matchPreview.decisionKind === "ambiguous"}
            <button
              class="btn-primary"
              on:click={() => payments.confirmSelectedPreviewCandidate()}
              disabled={$payments.loading}
            >
              Підтвердити вибраний варіант
            </button>
            <button class="btn-secondary" on:click={openManualPickerForCurrentPreview} disabled={$payments.loading}>
              Інший документ
            </button>
          {/if}
          <button class="btn-ghost" on:click={() => payments.closeMatchPreview()} disabled={$payments.loading}>
            Закрити preview
          </button>
        </div>
      </div>

      {#if $payments.matchPreview.decisionKind === "exact" && $payments.matchPreview.autoMatch}
        <div class="doc-row payment-row payment-row-matched">
          <div class="task-row-main">
            <div>
              <strong>{$payments.matchPreview.autoMatch.title}</strong>
              <p>{getDocumentKindLabel($payments.matchPreview.autoMatch.documentKind)} • {$payments.matchPreview.autoMatch.amountStr}</p>
            </div>
            <div class="task-row-meta">
              <span class="task-pill">Рекомендація</span>
              <span>Автопідтвердження доступне</span>
            </div>
          </div>
        </div>
      {:else if $payments.matchPreview.decisionKind === "ambiguous"}
        <div class="documents-list">
          {#each $payments.matchPreview.candidates as candidate}
            <div
              class="doc-row payment-row"
              class:payment-row-matched={$payments.selectedCandidateId === candidate.documentId}
            >
              <div class="task-row-main">
                <div>
                  <strong>{candidate.title}</strong>
                  <p>{getDocumentKindLabel(candidate.documentKind)} • {candidate.openAmountStr}</p>
                  <p>{getCandidateHint(candidate)}</p>
                </div>
                <div class="task-row-meta">
                  <span class="task-pill">Скоринг {candidate.totalScore.toFixed(2)}</span>
                  {#if $payments.selectedCandidateId === candidate.documentId}
                    <span class="payment-state payment-state-matched">Обраний варіант</span>
                  {/if}
                </div>
              </div>
              <div>
                <button
                  class="btn-secondary"
                  on:click={() => payments.selectPreviewCandidate(candidate.documentId)}
                  disabled={$payments.loading}
                >
                  {$payments.selectedCandidateId === candidate.documentId ? "Вибрано" : "Обрати"}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {:else if $payments.matchPreview.decisionKind === "split"}
        <div class="documents-list">
          {#each $payments.matchPreview.candidates as candidate}
            <div class="doc-row payment-row payment-row-matched">
              <div class="task-row-main">
                <div>
                  <strong>{candidate.title}</strong>
                  <p>{getDocumentKindLabel(candidate.documentKind)} вЂў {candidate.openAmountStr}</p>
                  <p>{getCandidateHint(candidate)}</p>
                </div>
                <div class="task-row-meta">
                  <span class="task-pill">Рекомендація для розподілу</span>
                </div>
              </div>
            </div>
          {/each}
        </div>
        <div class="editor-actions">
          <button class="btn-secondary" on:click={openManualPickerForCurrentPreview} disabled={$payments.loading}>
            Інший документ
          </button>
          <div class="empty-state-actions">
            <button class="btn-primary" type="button" on:click={focusImportButton}>Р†РјРїРѕСЂС‚СѓРІР°С‚Рё РІРёРїРёСЃРєСѓ</button>
          </div>
        </div>
      {:else}
        <div class="editor-items-empty">
          <strong>Автоматична звірка не знайшла точного документа</strong>
          <p>Перевірте референс платежу, контрагента або відкрийте ручний пошук документа.</p>
          <button class="btn-secondary" on:click={openManualPickerForCurrentPreview} disabled={$payments.loading}>
            Ручний пошук документа
          </button>
        </div>
      {/if}

      {#if $payments.manualPicker}
        <section class="editor-items-empty" data-testid="payments-manual-picker">
          <strong>Ручний вибір документа</strong>
          <p>Знайдіть акт або накладну за номером, назвою чи призначенням платежу.</p>
          <div class="editor-grid">
            <label class="editor-grid-span">
              Пошук
              <input
                value={$payments.manualPicker.query}
                on:input={onManualSearchInput}
                placeholder="ACT-001, INV-002, оплата..."
              />
            </label>
          </div>
          <div class="editor-actions">
            <button class="btn-secondary" on:click={() => payments.searchManualMatchCandidates()} disabled={$payments.loading}>
              Оновити пошук
            </button>
              <button class="btn-secondary" on:click={() => payments.addSelectedManualPickerCandidateToSplit()} disabled={$payments.loading}>
                Р”РѕРґР°С‚Рё РґРѕ СЂРѕР·РїРѕРґС–Р»Сѓ
              </button>
              <button
                class="btn-primary"
                on:click={() => payments.confirmManualPickerCandidate()}
                aria-describedby={!manualPickerCanConfirm && manualPickerDisabledReason ? "payments-manual-picker-hint" : undefined}
                disabled={$payments.loading || !manualPickerCanConfirm}
            >
              Підтвердити вибраний документ
            </button>
            <button class="btn-ghost" on:click={() => payments.closeManualMatchPicker()} disabled={$payments.loading}>
              Закрити пошук
            </button>
          </div>

          {#if !manualPickerCanConfirm && manualPickerDisabledReason}
            <p id="payments-manual-picker-hint">{manualPickerDisabledReason}</p>
          {/if}

          {#if $payments.manualPicker.candidates.length === 0}
            <p>За цим запитом кандидатів поки немає.</p>
          {:else}
            <div class="documents-list">
              {#each $payments.manualPicker.candidates as candidate}
                <div
                  class="doc-row payment-row"
                  class:payment-row-matched={$payments.manualPicker.selectedCandidateId === candidate.documentId}
                >
                  <div class="task-row-main">
                    <div>
                      <strong>{candidate.title}</strong>
                      <p>{getDocumentKindLabel(candidate.documentKind)} • {candidate.openAmountStr}</p>
                      <p>{getCandidateHint(candidate)}</p>
                    </div>
                    <div class="task-row-meta">
                      <span class="task-pill">Скоринг {candidate.totalScore.toFixed(2)}</span>
                    </div>
                  </div>
                  <div>
                    <button
                      class="btn-secondary"
                      on:click={() => payments.selectManualPickerCandidate(candidate.documentId)}
                      disabled={$payments.loading}
                    >
                      {$payments.manualPicker.selectedCandidateId === candidate.documentId ? "Вибрано" : "Обрати"}
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      {#if $payments.splitDraft}
        <section class="editor-items-empty" data-testid="payments-split-draft">
          <strong>Р§РµСЂРЅРµС‚РєР° СЂРѕР·РїРѕРґС–Р»Сѓ</strong>
          <p>
            РЎСѓРјР° РїР»Р°С‚РµР¶Сѓ: {$payments.splitDraft.paymentAmountStr}
            вЂў Р—Р°Р»РёС€РѕРє: {$payments.splitDraft.remainingAmountStr}
          </p>

          {#if $payments.splitDraft.allocations.length === 0}
            <p>Р”РѕРґР°Р№С‚Рµ РґРѕРєСѓРјРµРЅС‚Рё Р· manual picker, С‰РѕР± СЃС„РѕСЂРјСѓРІР°С‚Рё СЂРѕР·РїРѕРґС–Р».</p>
          {:else}
            <div class="documents-list">
              {#each $payments.splitDraft.allocations as allocation}
                <div class="doc-row payment-row">
                  <div class="task-row-main">
                    <div>
                      <strong>{allocation.title}</strong>
                      <p>{getDocumentKindLabel(allocation.documentKind)} вЂў Р—Р°Р»РёС€РѕРє РґРѕРєСѓРјРµРЅС‚Р°: {allocation.openAmountStr}</p>
                    </div>
                    <div class="task-row-meta">
                      <label>
                        <span>РЎСѓРјР°</span>
                        <input
                          value={allocation.amount}
                          on:input={(event) => onSplitAllocationInput(allocation.documentId, event)}
                        />
                      </label>
                    </div>
                  </div>
                  <div>
                    <button class="btn-ghost" on:click={() => payments.removeSplitAllocation(allocation.documentId)} disabled={$payments.loading}>
                      РџСЂРёР±СЂР°С‚Рё
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <div class="editor-actions">
            <button class="btn-primary" on:click={() => payments.confirmSplitDraft()} disabled={$payments.loading || $payments.splitDraft.allocations.length === 0}>
              РџС–РґС‚РІРµСЂРґРёС‚Рё СЂРѕР·РїРѕРґС–Р»
            </button>
          </div>
        </section>
      {/if}
    </section>
  {/if}

  <div class="payments-groups">
    <section class="payments-group payments-group-unmatched" data-testid="payments-unmatched-group">
      <div class="payments-group-header">
        <div>
          <strong>Потребують звірки</strong>
          <p>Починайте саме з цих рухів: вони ще не пов'язані з документами.</p>
        </div>
        <span class="payment-group-count">{unmatchedPayments.length}</span>
      </div>

      {#if $payments.initialLoading}
        <SkeletonRow count={3} />
      {:else if unmatchedPayments.length === 0}
        <div class="editor-items-empty">
          <span class="empty-state-eyebrow">Додайте перший рух</span>
          <strong>Ще немає жодного платежу</strong>
          <p>Імпортуйте виписку або створіть ручний платіж, щоб почати звірку руху грошей.</p>
        </div>
      {:else}
        <div class="documents-list">
          {#each unmatchedPayments as item (item.id)}
            <div class="doc-row payment-row payment-row-unmatched">
              <button class="task-row-main payment-row-main" on:click={() => payments.openEditor(item)}>
                <div>
                  <strong>{item.date} - {item.counterparty || "Без контрагента"}</strong>
                  <p>{item.account || "Банк не вказано"}</p>
                </div>
                <div class="task-row-meta">
                  <span class="task-pill">{item.direction === "in" ? "Надходження" : "Витрата"}</span>
                  <span class="money-value" data-negative={item.amountStr.trim().startsWith("-")}>{item.amountStr}</span>
                  <span class="payment-state payment-state-unmatched">{getPaymentStateLabel(item.matchedDoc)}</span>
                </div>
              </button>
              <div class="payment-row-actions">
                <button
                  class="btn-primary"
                  on:click={() => payments.reconcile(item.id)}
                  disabled={isPaymentBusy(item.id)}
                >
                  {isPaymentBusy(item.id) ? "Зводимо..." : "Звести"}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section class="payments-group" data-testid="payments-matched-group">
      <div class="payments-group-header">
        <div>
          <strong>Вже зведені</strong>
          <p>Тут лишаються платежі, які вже мають зв'язок із документом і потребують лише контролю.</p>
        </div>
        <span class="payment-group-count">{matchedPayments.length}</span>
      </div>

      {#if $payments.initialLoading}
        <SkeletonRow count={3} />
      {:else if matchedPayments.length === 0}
        <div class="editor-items-empty">
          <strong>Ще немає зведених платежів</strong>
          <p>Проведіть першу звірку в лівому блоці, щоб тут з'явився готовий результат.</p>
        </div>
      {:else}
        <div class="documents-list">
          {#each matchedPayments as item (item.id)}
            <div class="doc-row payment-row payment-row-matched">
              <button class="task-row-main payment-row-main" on:click={() => payments.openEditor(item)}>
                <div>
                  <strong>{item.date} - {item.counterparty || "Без контрагента"}</strong>
                  <p>{item.account || "Банк не вказано"}</p>
                </div>
                <div class="task-row-meta">
                  <span class="task-pill">{item.direction === "in" ? "Надходження" : "Витрата"}</span>
                  <span class="money-value" data-negative={item.amountStr.trim().startsWith("-")}>{item.amountStr}</span>
                  <span class="payment-state payment-state-matched">{getPaymentStateLabel(item.matchedDoc)}</span>
                </div>
              </button>
              <div class="payment-row-actions">
                <button
                  class="btn-secondary"
                  on:click={() => payments.unreconcile(item.id)}
                  disabled={isPaymentBusy(item.id)}
                >
                  {isPaymentBusy(item.id) ? "Знімаємо..." : "Зняти зведення"}
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</section>

{#if $payments.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <h3>{$payments.editor.id ? "Редагувати платіж" : "Новий платіж"}</h3>
        <p>Картка платежу</p>
      </div>
      <div class="editor-actions">
        <button class="btn-primary" on:click={() => payments.save()} disabled={busySave}>Зберегти</button>
        <button class="btn-ghost" on:click={() => payments.closeEditor()} disabled={busySave}>Закрити</button>
      </div>
    </div>

    <div class="chain-panel">
      <div class="chain-panel-header">
        <div>
          <strong>Що перевірити перед збереженням</strong>
          <p>Перевірте напрям, суму, контрагента та референс, щоб звірка не губилася після імпорту.</p>
        </div>
        <div class="chain-summary">
          <div class="chain-summary-block">
            <span>Напрям</span>
            <strong>{$payments.editor.direction === "income" ? "Надходження" : "Витрата"}</strong>
          </div>
          <div class="chain-summary-block">
            <span>Пов'язаний документ</span>
            <strong>{$payments.editor.description || "Ще не вказано"}</strong>
          </div>
        </div>
      </div>
    </div>

    <div class="editor-grid payment-editor-grid">
      <label>
        Дата
        <input type="date" value={$payments.editor.date} on:input={(event) => onPaymentFieldChange("date", event)} />
      </label>
      <label>
        Напрям
        <select value={$payments.editor.direction} on:change={(event) => onPaymentFieldChange("direction", event)}>
          <option value="income">Надходження</option>
          <option value="expense">Витрата</option>
        </select>
      </label>
      <label>
        Сума
        <input value={$payments.editor.amount} on:input={(event) => onPaymentFieldChange("amount", event)} />
      </label>
      <label>
        Контрагент
        <select value={$payments.editor.counterpartyId} on:change={(event) => onPaymentFieldChange("counterpartyId", event)}>
          <option value="">- Без контрагента -</option>
          {#each $payments.list?.counterparties ?? [] as cp}
            <option value={cp.id}>{cp.name}</option>
          {/each}
        </select>
      </label>
      <label>
        Референс платежу
        <input value={$payments.editor.reference} on:input={(event) => onPaymentFieldChange("reference", event)} />
      </label>
      <label>
        Пов'язаний документ
        <input value={$payments.editor.description} on:input={(event) => onPaymentFieldChange("description", event)} />
      </label>
      <label class="editor-grid-span">
        Банк
        <input value={$payments.editor.bankName} on:input={(event) => onPaymentFieldChange("bankName", event)} />
      </label>
    </div>
  </section>
{/if}

<style>
  .create-strip-card,
  .payments-group,
  .flow-banner,
  .chain-panel,
  .editor-sheet {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--radius-3xl);
    border: 1px solid var(--border-hairline);
    background: var(--bg-card);
  }

  .create-strip-card,
  .flow-banner,
  .chain-panel {
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--accent-soft) 32%, transparent), transparent 74%),
      var(--bg-card);
  }

  .create-strip-header,
  .payments-group-header,
  .chain-panel-header,
  .editor-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .create-strip-header p,
  .payments-group-header p,
  .chain-panel-header p,
  .payment-action-copy p,
  .flow-banner p {
    margin: 6px 0 0;
    color: var(--text-muted);
  }

  .payment-actions-grid,
  .payments-groups,
  .task-kpis,
  .documents-list,
  .editor-sheet {
    display: grid;
    gap: 16px;
  }

  .payment-actions-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .task-kpis {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .payment-action-card,
  .task-kpi-card {
    display: grid;
    gap: 10px;
    padding: 16px;
    border-radius: var(--radius-2xl);
    border: 1px solid var(--border-hairline);
    background: color-mix(in srgb, var(--bg-subtle) 72%, var(--bg-card));
  }

  .payment-action-card-primary,
  .task-kpi-card-alert,
  .payments-group-unmatched {
    border-color: color-mix(in srgb, var(--accent) 22%, var(--border-hairline));
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--accent-soft) 36%, transparent), transparent 80%),
      var(--bg-card);
  }

  .payment-action-copy span,
  .payment-group-count,
  .task-kpi-card span,
  .chain-summary-block span {
    font-size: var(--font-sm);
    color: var(--text-muted);
  }

  .payment-action-buttons,
  .payment-row-actions,
  .editor-actions,
  .task-row-meta {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }

  .payment-group-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 40px;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-subtle) 82%, var(--bg-card));
    font-weight: 700;
  }

  .doc-kind-badge,
  .payment-state {
    display: inline-flex;
    align-items: center;
    min-height: 30px;
    padding: 0 10px;
    border-radius: 999px;
    font-size: var(--font-sm);
    font-weight: 600;
  }

  .doc-kind-badge {
    background: color-mix(in srgb, var(--bg-subtle) 88%, var(--bg-card));
    color: var(--text-muted);
  }

  .payment-row {
    display: flex;
    gap: 16px;
    justify-content: space-between;
    align-items: center;
    padding: 12px;
    border-radius: var(--radius-2xl);
    border: 1px solid var(--border-hairline);
    background: color-mix(in srgb, var(--bg-subtle) 62%, var(--bg-card));
  }

  .payment-row-unmatched {
    border-color: color-mix(in srgb, var(--danger, #c2410c) 24%, var(--border-hairline));
  }

  .payment-row-matched {
    border-color: color-mix(in srgb, var(--accent) 20%, var(--border-hairline));
  }

  .payment-row-main {
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .payment-row-main p,
  .task-row-meta,
  .editor-items-empty p {
    margin: 6px 0 0;
  }

  .payment-state-unmatched {
    background: color-mix(in srgb, var(--danger, #c2410c) 12%, var(--bg-card));
    color: color-mix(in srgb, var(--danger, #c2410c) 72%, var(--text));
  }

  .payment-state-matched {
    background: color-mix(in srgb, var(--accent-soft) 70%, var(--bg-card));
    color: var(--accent-text);
  }

  .editor-items-empty {
    display: grid;
    gap: 10px;
    padding: 20px;
    border-radius: var(--radius-2xl);
    border: 1px dashed color-mix(in srgb, var(--accent) 26%, var(--border-hairline));
    background: color-mix(in srgb, var(--bg-subtle) 72%, var(--bg-card));
  }

  .chain-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .chain-summary-block {
    display: grid;
    gap: 6px;
    padding: 14px 16px;
    border-radius: var(--radius-2xl);
    border: 1px solid var(--border-hairline);
    background: color-mix(in srgb, var(--bg-subtle) 76%, var(--bg-card));
  }

  .editor-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }

  .editor-grid label {
    display: grid;
    gap: 8px;
  }

  .editor-grid-span {
    grid-column: 1 / -1;
  }

  @media (max-width: 1080px) {
    .payment-actions-grid,
    .task-kpis,
    .editor-grid,
    .chain-summary {
      grid-template-columns: 1fr;
    }

    .create-strip-header,
    .payments-group-header,
    .chain-panel-header,
    .editor-header,
    .payment-row {
      flex-direction: column;
      align-items: flex-start;
    }
  }

  .payment-import-preview-path {
    font-size: 0.85em;
    color: var(--color-text-muted, #888);
    margin: 0.25rem 0 0 0;
    overflow-wrap: anywhere;
  }

  .payment-import-preview-summary {
    display: flex;
    flex-wrap: wrap;
    gap: 1.5rem;
    list-style: none;
    padding: 0;
    margin: 1rem 0 0.75rem 0;
  }

  .payment-import-preview-summary li {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .payment-import-preview-summary li span {
    font-size: 0.78em;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-muted, #888);
  }

  .payment-import-preview-summary li strong {
    font-size: 1.25rem;
  }

  .payment-import-preview-summary li.payment-import-preview-conflict strong {
    color: var(--color-danger, #c0392b);
  }

  .payment-import-preview-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.75rem;
    font-size: 0.92rem;
  }

  .payment-import-preview-table thead {
    background: var(--color-surface-alt, #f3f5f8);
  }

  .payment-import-preview-table th,
  .payment-import-preview-table td {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--color-border-subtle, #e6e8ec);
    vertical-align: top;
  }

  .payment-import-preview-table tr.is-skipped {
    color: var(--color-text-muted, #888);
  }

  .payment-import-preview-more,
  .payment-import-preview-empty {
    margin: 0.5rem 0 0 0;
    font-size: 0.88rem;
    color: var(--color-text-muted, #888);
  }
</style>
