<script lang="ts">
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import {
    COUNTERPARTIES_COPY,
    EDITOR_DIRTY_COPY
  } from "../config/ui";
  import {
    getCounterpartyFinancialSummary,
    getCounterpartyLastContactLabel,
    getCounterpartyOverdueDocumentsLabel,
    getCounterpartyRiskLabel,
    getCounterpartyScenarioDescription,
    getCounterpartyScenarioTitle
  } from "../counterpartyPresentation";
  import { isFormattedMoneyNegative } from "../money";
  import { counterpartiesStore } from "../stores/counterparties";
  import { documentsStore } from "../stores/documents";
  import { navigationStore } from "../stores/navigation";

  const counterparties = counterpartiesStore;
  const documents = documentsStore;
  const navigation = navigationStore;
  let pendingDirtyClose = false;

  function closeEditor(force = false) {
    const result = counterparties.closeEditor(force);
    if (result && result.ok === false && result.reason === "dirty") {
      pendingDirtyClose = true;
      return result;
    }

    pendingDirtyClose = false;
    return result;
  }

  function onCounterpartySearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void counterparties.load(input.value);
  }

  function onCounterpartyFieldChange(field: string, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
    counterparties.updateFormField(
      field as
        | "name"
        | "edrpou"
        | "ipn"
        | "iban"
        | "address"
        | "phone"
        | "email"
        | "notes",
      input.value
    );
  }

  function requestCloseEditor() {
    closeEditor();
  }

  function confirmDiscardChanges() {
    closeEditor(true);
  }

  function cancelDiscardChanges() {
    pendingDirtyClose = false;
  }

  function onEditorBackdropClick() {
    requestCloseEditor();
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if ($counterparties.editor && event.key === "Escape") {
      requestCloseEditor();
    }
  }

  let pendingArchive = false;

  function onArchiveCurrent() {
    pendingArchive = true;
  }

  function confirmArchive() {
    pendingArchive = false;
    counterparties.archiveCurrent();
  }

  function cancelArchive() {
    pendingArchive = false;
  }

  $: if (!$counterparties.editor && pendingDirtyClose) {
    pendingDirtyClose = false;
  }


</script>

<svelte:window on:keydown={onWindowKeydown} />

<section
  class="panel"
  data-testid="counterparties-screen"
  inert={$counterparties.editor ? true : undefined}
  aria-hidden={$counterparties.editor ? "true" : undefined}
>
  <div class="panel-header">
    <div>
      <h2>Контрагенти</h2>
      <p>{$counterparties.screen?.items.length ?? 0} записів</p>
    </div>
    <div class="panel-actions">
      <button class="btn-primary" on:click={() => counterparties.openEditor()}>Новий контрагент</button>
    </div>
  </div>

  {#if $counterparties.message}
    <p class="message">{$counterparties.message}</p>
  {/if}

  {#if $counterparties.error}
    <p class="error">{$counterparties.error}</p>
  {/if}

  {#if $counterparties.loading}
    <p class="message">{COUNTERPARTIES_COPY.loadingMessage}</p>
  {/if}

  <div class="counterparties-layout">
    <div class="counterparties-list-wrap">
      <div class="counterparties-search-bar">
        <input placeholder={COUNTERPARTIES_COPY.searchPlaceholder} on:input={onCounterpartySearch} />
      </div>
      <div class="counterparties-scroll">
        <div class="counterparties-list" data-testid="counterparties-list">
          {#if $counterparties.initialLoading}
            <SkeletonRow count={6} />
          {:else}
            {#each $counterparties.screen?.items ?? [] as item}
              <button
                class:active={$counterparties.selectedId === item.id}
                class="counterparty-row"
                on:click={() => counterparties.open(item.id)}
              >
                <div class="counterparty-row-main">
                  <strong>{item.name}</strong>
                  <p>{item.edrpou || "Без ЄДРПОУ"}</p>
                </div>
                <div class="counterparty-row-meta">
                  <span class="money-value" data-negative={isFormattedMoneyNegative(item.balanceStr)}>{item.balanceStr}</span>
                  <span class="task-pill">{item.kind}</span>
                  {#if item.overdueCount > 0}
                    <span class="risk-chip risk-chip-danger">{item.overdueCount} простр.</span>
                  {/if}
                </div>
              </button>
            {/each}
          {/if}
        </div>
      </div>
    </div>

    <div class="counterparty-detail">
      {#if $counterparties.initialLoading}
        <div class="empty-screen empty-state-card compact cp-empty-padded" aria-live="polite">
          <strong>{COUNTERPARTIES_COPY.loadingTitle}</strong>
          <p>{COUNTERPARTIES_COPY.loadingDescription}</p>
        </div>
      {:else if $counterparties.detail}
        <div data-testid="counterparty-detail">
          <div class="counterparty-detail-section-label" data-testid="counterparty-detail-section-label">
            Деталі контрагента
          </div>
          <div class="counterparty-detail-main">
            <div class="counterparty-detail-header">
              <div>
                <div class="scenario-eyebrow">{$counterparties.detail.info.kind === "ФОП" ? "Фізична особа-підприємець" : "Юридична особа"}</div>
                <h3>{$counterparties.detail.info.name}</h3>
                <div class="counterparty-overview-badges">
                  <span class="task-pill">{$counterparties.detail.info.kind}</span>
                  <span
                    class:risk-chip-danger={$counterparties.detail.info.overdueCount > 0}
                    class:risk-chip-ok={$counterparties.detail.info.overdueCount <= 0}
                    class="risk-chip"
                  >
                    {getCounterpartyRiskLabel($counterparties.detail.info.overdueCount)}
                  </span>
                </div>
              </div>
              <div class="editor-actions">
                <button class="btn-secondary" on:click={() => counterparties.openEditor($counterparties.detail?.info.id)}>
                  Редагувати
                </button>
                <button class="btn-primary" on:click={() => counterparties.createDocument()}>
                  Створити документ
                </button>
                <button class="btn-danger" type="button" on:click={onArchiveCurrent}>
                  Архівувати
                </button>
              </div>
            </div>
          </div>

          {#if pendingArchive}
            <div
              class="confirm-delete-banner cp-confirm-banner"
              role="alertdialog"
              aria-live="assertive"
              aria-labelledby="cp-confirm-archive-title"
              data-testid="counterparties-confirm-archive-banner"
            >
              <div>
                <strong id="cp-confirm-archive-title">Архівувати контрагента?</strong>
                <p>{COUNTERPARTIES_COPY.archiveConfirm}</p>
              </div>
              <div class="editor-dirty-actions">
                <button type="button" class="btn-ghost btn-sm" on:click={cancelArchive}>Скасувати</button>
                <button type="button" class="btn-danger btn-sm" on:click={confirmArchive} data-testid="counterparties-confirm-archive-confirm">Архівувати</button>
              </div>
            </div>
          {/if}

          <!-- Flat metric strip -->
          <div class="counterparty-metric-strip">
            <div class="counterparty-metric">
              <div class="counterparty-metric-label">Баланс</div>
              <div class="counterparty-metric-value" class:is-danger={$counterparties.detail.info.balanceIsNegative}>
                {$counterparties.detail.info.balanceStr}
              </div>
              <div class="counterparty-metric-sub">{$counterparties.detail.info.balanceIsNegative ? "ми винні" : "нам винні"}</div>
            </div>
            <div class="counterparty-metric-divider"></div>
            <div class="counterparty-metric">
              <div class="counterparty-metric-label">Документів</div>
              <div class="counterparty-metric-value">{$counterparties.detail.info.docCount}</div>
              <div class="counterparty-metric-sub">{$counterparties.detail.documents.length} активних</div>
            </div>
            <div class="counterparty-metric-divider"></div>
            <div class="counterparty-metric">
              <div class="counterparty-metric-label">Прострочено</div>
              <div class="counterparty-metric-value" class:is-danger={$counterparties.detail.info.overdueCount > 0}>
                {$counterparties.detail.info.overdueCount}
              </div>
              <div class="counterparty-metric-sub">{$counterparties.detail.info.overdueCount > 0 ? $counterparties.detail.info.overdueAmountStr : "немає"}</div>
            </div>
            <div class="counterparty-metric-divider"></div>
            <div class="counterparty-metric">
              <div class="counterparty-metric-label">Останній контакт</div>
              <div class="counterparty-metric-value counterparty-metric-value-sm">{$counterparties.detail.info.lastContactDate || "—"}</div>
              <div class="counterparty-metric-sub">{getCounterpartyLastContactLabel($counterparties.detail.info.lastContactDays)}</div>
            </div>
          </div>

          <div class="counterparty-detail-scroll">
          <div class="counterparty-scenario-grid" data-testid="counterparty-scenario">
            <article class="scenario-card">
              <span class="scenario-eyebrow">Хто це</span>
              <strong>{$counterparties.detail.info.kind} {$counterparties.detail.info.name}</strong>
              <div class="scenario-facts">
                <div>
                  <span>Директор</span>
                  <strong>{$counterparties.detail.info.director || "—"}</strong>
                </div>
                <div>
                  <span>Банк</span>
                  <strong>{$counterparties.detail.info.bank || "—"}</strong>
                </div>
                <div>
                  <span>VAT</span>
                  <strong>{$counterparties.detail.info.vat || "—"}</strong>
                </div>
                <div>
                  <span>IBAN</span>
                  <strong>{$counterparties.detail.info.iban || "—"}</strong>
                </div>
              </div>
            </article>

            <article class="scenario-card">
              <span class="scenario-eyebrow">Фінансовий стан</span>
              <strong>{$counterparties.detail.info.balanceStr}</strong>
              <p>{getCounterpartyFinancialSummary(
                $counterparties.detail.info.balanceIsNegative,
                $counterparties.detail.info.overdueCount
              )}</p>
              <div class="scenario-facts">
                <div>
                  <span>Баланс</span>
                  <strong>{$counterparties.detail.info.balanceStr}</strong>
                </div>
                <div>
                  <span>Прострочено</span>
                  <strong>{getCounterpartyOverdueDocumentsLabel($counterparties.detail.info.overdueCount)}</strong>
                </div>
                <div>
                  <span>Сума прострочки</span>
                  <strong>{$counterparties.detail.info.overdueAmountStr}</strong>
                </div>
                <div>
                  <span>Останній контакт {$counterparties.detail.info.lastContactDate}</span>
                  <strong>{getCounterpartyLastContactLabel($counterparties.detail.info.lastContactDays)}</strong>
                </div>
              </div>
            </article>

            <article class="scenario-card">
              <span class="scenario-eyebrow">Документи</span>
              <strong>{$counterparties.detail.info.docCount} в роботі</strong>
              <div class="linked-list">
                {#if $counterparties.detail.documents.length > 0}
                  {#each $counterparties.detail.documents as item}
                    <button
                      class="linked-row"
                      on:click={() => {
                        navigation.go("documents");
                        void documents.open(item.id);
                      }}
                    >
                      <span>{item.number}</span>
                      <span>{item.amountStr}</span>
                    </button>
                  {/each}
                {:else}
                  <div class="linked-empty">Ще немає документів. Найкращий наступний крок - створити перший документ.</div>
                {/if}
              </div>
            </article>

            <article class="scenario-card">
              <span class="scenario-eyebrow">Платежі</span>
              <strong>{$counterparties.detail.payments.length} останніх рухів</strong>
              <div class="linked-list">
                {#if $counterparties.detail.payments.length > 0}
                  {#each $counterparties.detail.payments as payment}
                    <div class="linked-row static">
                      <span>{payment.date} • {payment.account}</span>
                      <span>{payment.amountStr}</span>
                    </div>
                  {/each}
                {:else}
                  <div class="linked-empty">По контрагенту ще немає рухів коштів. Після створення документа зручніше повернутися сюди й перевірити оплату.</div>
                {/if}
              </div>
            </article>

            <article class="scenario-card scenario-card-accent">
              <span class="scenario-eyebrow">Наступна дія</span>
              <strong>{getCounterpartyScenarioTitle(
                $counterparties.detail.info.overdueCount,
                $counterparties.detail.info.lastContactDays,
                $counterparties.detail.info.docCount
              )}</strong>
              <p>{getCounterpartyScenarioDescription(
                $counterparties.detail.info.overdueCount,
                $counterparties.detail.info.overdueAmountStr,
                $counterparties.detail.info.lastContactDays,
                $counterparties.detail.info.docCount
              )}</p>
              <div class="scenario-facts">
                <div>
                  <span>Контакт</span>
                  <strong>{$counterparties.detail.info.lastContactDate} • {getCounterpartyLastContactLabel($counterparties.detail.info.lastContactDays)}</strong>
                </div>
                <div>
                  <span>Документів</span>
                  <strong>{$counterparties.detail.info.docCount}</strong>
                </div>
              </div>
            </article>
          </div>
          </div><!-- counterparty-detail-scroll -->
        </div>
      {:else}
        <div class="empty-screen empty-state-card compact cp-empty-padded" data-testid="counterparties-empty-state">
          <strong>{COUNTERPARTIES_COPY.emptyTitle}</strong>
          <p>
            {COUNTERPARTIES_COPY.emptyDescription}
          </p>
          <button class="btn-primary" on:click={() => counterparties.openEditor()}>Новий контрагент</button>
        </div>
      {/if}
    </div>
  </div>
</section>

{#if $counterparties.editor}
  <button
    type="button"
    class="editor-backdrop"
    aria-label="Закрити редактор"
    data-testid="counterparties-editor-backdrop"
    on:click={onEditorBackdropClick}
  ></button>
  <section class="editor-sheet" role="dialog" aria-modal="true">
    {#if pendingDirtyClose}
      <div
        class="editor-dirty-banner"
        role="alertdialog"
        aria-live="assertive"
        aria-labelledby="counterparties-dirty-banner-title"
        data-testid="counterparties-dirty-banner"
      >
        <div>
          <strong id="counterparties-dirty-banner-title">{EDITOR_DIRTY_COPY.dirtyTitle}</strong>
          <p>{EDITOR_DIRTY_COPY.dirtyDescription}</p>
        </div>
        <div class="editor-dirty-actions">
          <button
            type="button"
            class="btn-ghost btn-sm"
            on:click={cancelDiscardChanges}
            data-testid="counterparties-dirty-banner-cancel"
          >
            {EDITOR_DIRTY_COPY.dirtyStay}
          </button>
          <button
            type="button"
            class="btn-danger btn-sm"
            on:click={confirmDiscardChanges}
            data-testid="counterparties-dirty-banner-discard"
          >
            {EDITOR_DIRTY_COPY.dirtyDiscard}
          </button>
        </div>
      </div>
    {/if}
    <div class="editor-header">
      <div>
        <h3>{$counterparties.editor.form.title}</h3>
        <p>Картка контрагента</p>
      </div>
      <div class="editor-actions">
        <button class="btn-primary" on:click={() => counterparties.save()}>Зберегти</button>
        <button class="btn-ghost" on:click={requestCloseEditor}>Закрити</button>
      </div>
    </div>

    <div class="editor-grid cp-editor-grid">
      <label>
        Назва
        <input value={$counterparties.editor.form.name} on:input={(event) => onCounterpartyFieldChange("name", event)} />
      </label>
      <label>
        ЄДРПОУ
        <input value={$counterparties.editor.form.edrpou} on:input={(event) => onCounterpartyFieldChange("edrpou", event)} />
      </label>
      <label>
        ІПН
        <input value={$counterparties.editor.form.ipn} on:input={(event) => onCounterpartyFieldChange("ipn", event)} />
      </label>
      <label>
        IBAN
        <input value={$counterparties.editor.form.iban} on:input={(event) => onCounterpartyFieldChange("iban", event)} />
      </label>
      <label>
        Телефон
        <input value={$counterparties.editor.form.phone} on:input={(event) => onCounterpartyFieldChange("phone", event)} />
      </label>
      <label>
        Email
        <input value={$counterparties.editor.form.email} on:input={(event) => onCounterpartyFieldChange("email", event)} />
      </label>
      <label class="editor-grid-span">
        Адреса
        <input value={$counterparties.editor.form.address} on:input={(event) => onCounterpartyFieldChange("address", event)} />
      </label>
      <label class="editor-grid-span">
        Примітки
        <textarea rows="4" value={$counterparties.editor.form.notes} on:input={(event) => onCounterpartyFieldChange("notes", event)}></textarea>
      </label>
    </div>
  </section>
{/if}

<style>
  .counterparty-overview-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
  }
</style>

