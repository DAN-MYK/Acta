<script lang="ts">
  import type { PaymentImportPreviewDto } from "../../types";

  export let preview: PaymentImportPreviewDto;
  export let stale = false;
  export let busyImportPick = false;
  export let busyImportCommit = false;
  export let onRefresh: () => void;
  export let onCommit: () => void;
  export let onCancel: () => void;
</script>

<section class="chain-panel" data-testid="payments-import-preview" aria-label="Попередній перегляд імпорту виписки">
  <div class="chain-panel-header">
    <div>
      <strong>Попередній перегляд: {preview.bankName || "виписка"}</strong>
      <p>{preview.message}</p>
      <p class="payment-import-preview-path"><code>{preview.path}</code></p>
      {#if stale}
        <div class="status-banner is-warning" role="status" aria-live="polite" data-testid="payments-import-preview-stale">
          <div>
            <strong>Файл виписки змінився</strong>
            <p>Перечитайте виписку, щоб оновити план імпорту перед підтвердженням.</p>
          </div>
        </div>
      {/if}
    </div>
    <div class="editor-actions">
      {#if stale}
        <button
          class="btn-secondary"
          on:click={onRefresh}
          disabled={busyImportPick || busyImportCommit}
        >
          Перечитати файл
        </button>
      {/if}
      <button
        class="btn-primary"
        on:click={onCommit}
        disabled={busyImportCommit || preview.willCreate === 0 || stale}
      >
        {busyImportCommit
          ? "Імпортуємо..."
          : preview.willCreate === 0
          ? "Немає нових платежів"
          : `Імпортувати ${preview.willCreate} платежі(ів)`}
      </button>
      <button
        class="btn-ghost"
        on:click={onCancel}
        disabled={busyImportCommit}
      >
        Скасувати
      </button>
    </div>
  </div>

  <ul class="payment-import-preview-summary">
    <li><span>Розпізнано рядків</span><strong>{preview.parsed}</strong></li>
    <li><span>Буде створено</span><strong>{preview.willCreate}</strong></li>
    <li><span>Уже існує (skip)</span><strong>{preview.willSkip}</strong></li>
    {#if preview.conflicts > 0}
      <li class="payment-import-preview-conflict"><span>Конфлікти</span><strong>{preview.conflicts}</strong></li>
    {/if}
  </ul>

  {#if preview.rows.length > 0}
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
        {#each preview.rows.slice(0, 25) as row, idx (idx)}
          <tr class:is-skipped={row.action === "skip"}>
            <td>{row.action === "create" ? "Нове" : "Пропуск"}</td>
            <td>{row.bankRef || "—"}</td>
            <td>{row.description || "—"}</td>
            <td>{row.note}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if preview.rows.length > 25}
      <p class="payment-import-preview-more">
        Показано перші 25 з {preview.rows.length} рядків.
      </p>
    {/if}
  {:else}
    <p class="payment-import-preview-empty">У файлі не знайдено жодного рядка виписки.</p>
  {/if}
</section>

<style>
  .chain-panel {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--acta-color-accent-soft) 32%, transparent), transparent 74%),
      var(--acta-color-bg-elevated);
  }

  .chain-panel-header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }

  .chain-panel-header p {
    margin: 6px 0 0;
    color: var(--acta-color-text-muted);
  }

  .editor-actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
  }

  .payment-import-preview-path {
    font-size: 0.85em;
    color: var(--acta-color-text-muted);
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
    color: var(--acta-color-text-muted);
  }

  .payment-import-preview-summary li strong {
    font-size: 1.25rem;
  }

  .payment-import-preview-summary li.payment-import-preview-conflict strong {
    color: var(--acta-color-danger);
  }

  .payment-import-preview-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.75rem;
    font-size: 0.92rem;
  }

  .payment-import-preview-table thead {
    background: var(--acta-color-bg-subtle);
  }

  .payment-import-preview-table th,
  .payment-import-preview-table td {
    text-align: left;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--acta-color-border);
    vertical-align: top;
  }

  .payment-import-preview-table tr.is-skipped {
    color: var(--acta-color-text-muted);
  }

  .payment-import-preview-more,
  .payment-import-preview-empty {
    margin: 0.5rem 0 0 0;
    font-size: 0.88rem;
    color: var(--acta-color-text-muted);
  }

  @media (max-width: 1080px) {
    .chain-panel-header {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
