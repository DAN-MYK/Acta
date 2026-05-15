<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import Modal from './Modal.svelte';
  import type { CounterpartyDraftFormDto } from '../types';

  export let isOpen: boolean;
  export let mode: 'create' | 'edit';
  export let form: CounterpartyDraftFormDto | null;
  export let loading: boolean = false;
  export let isDirty: boolean = false;
  export let showCloseConfirm: boolean = false;

  const dispatch = createEventDispatcher<{
    close: void;
    save: void;
    closeConfirmed: void;
    closeCancelled: void;
    fieldChange: { field: keyof CounterpartyDraftFormDto; value: string };
  }>();

  $: title = mode === 'create' ? 'Новий контрагент' : 'Редагування контрагента';

  function handleClose() {
    dispatch('close');
  }

  function handleSave() {
    dispatch('save');
  }

  function handleCloseConfirmed() {
    dispatch('closeConfirmed');
  }

  function handleCloseCancelled() {
    dispatch('closeCancelled');
  }

  function handleFieldChange(field: keyof CounterpartyDraftFormDto, value: string) {
    dispatch('fieldChange', { field, value });
  }
</script>

<Modal open={isOpen} {title} on:close={handleClose}>
  <div class="cp-modal-body">
    {#if showCloseConfirm}
      <div class="cp-dirty-confirm" data-testid="cp-modal-dirty-confirm">
        <p class="cp-dirty-message">Є незбережені зміни. Закрити без збереження?</p>
        <div class="cp-dirty-actions">
          <button class="btn-secondary" on:click={handleCloseCancelled}>Залишитись</button>
          <button class="btn-danger" on:click={handleCloseConfirmed}>Так, закрити</button>
        </div>
      </div>
    {/if}

    {#if form}
      <div class="cp-form-grid">
        <div class="cp-form-field">
          <label for="cp-name">Назва</label>
          <input
            id="cp-name"
            type="text"
            value={form.name}
            on:input={(e) => handleFieldChange('name', e.currentTarget.value)}
            disabled={loading}
          />
        </div>

        <div class="cp-form-field">
          <label for="cp-edrpou">ЄДРПОУ</label>
          <input
            id="cp-edrpou"
            type="text"
            value={form.edrpou}
            on:input={(e) => handleFieldChange('edrpou', e.currentTarget.value)}
            disabled={loading}
          />
        </div>

        <div class="cp-form-field">
          <label for="cp-ipn">ІПН</label>
          <input
            id="cp-ipn"
            type="text"
            value={form.ipn}
            on:input={(e) => handleFieldChange('ipn', e.currentTarget.value)}
            disabled={loading}
          />
        </div>

        <div class="cp-form-field">
          <label for="cp-iban">IBAN</label>
          <input
            id="cp-iban"
            type="text"
            value={form.iban}
            on:input={(e) => handleFieldChange('iban', e.currentTarget.value)}
            disabled={loading}
          />
        </div>

        <div class="cp-form-field">
          <label for="cp-phone">Телефон</label>
          <input
            id="cp-phone"
            type="text"
            value={form.phone}
            on:input={(e) => handleFieldChange('phone', e.currentTarget.value)}
            disabled={loading}
          />
        </div>

        <div class="cp-form-field">
          <label for="cp-email">Email</label>
          <input
            id="cp-email"
            type="email"
            value={form.email}
            on:input={(e) => handleFieldChange('email', e.currentTarget.value)}
            disabled={loading}
          />
        </div>

        <div class="cp-form-field cp-span-2">
          <label for="cp-address">Адреса</label>
          <input
            id="cp-address"
            type="text"
            value={form.address}
            on:input={(e) => handleFieldChange('address', e.currentTarget.value)}
            disabled={loading}
          />
        </div>

        <div class="cp-form-field cp-span-2">
          <label for="cp-notes">Нотатки</label>
          <textarea
            id="cp-notes"
            value={form.notes}
            on:input={(e) => handleFieldChange('notes', e.currentTarget.value)}
            disabled={loading}
            rows={3}
          ></textarea>
        </div>
      </div>
    {/if}
  </div>

  <div slot="footer" class="cp-modal-footer">
    <button class="btn-secondary" on:click={handleClose} disabled={loading}>Скасувати</button>
    <button class="btn-primary" on:click={handleSave} disabled={loading}>Зберегти</button>
  </div>
</Modal>

<style>
  .cp-modal-body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .cp-dirty-confirm {
    padding: 12px 16px;
    background: var(--acta-color-warning-bg, #fef3c7);
    border: 1px solid var(--acta-color-warning-border, #f59e0b);
    border-radius: var(--acta-radius-md, 6px);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .cp-dirty-message {
    margin: 0;
    font-size: 14px;
    color: var(--acta-color-text, #111);
  }

  .cp-dirty-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .cp-form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .cp-form-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .cp-span-2 {
    grid-column: span 2;
  }

  label {
    font-size: 13px;
    font-weight: 500;
    color: var(--acta-color-text-muted, #6b7280);
  }

  input,
  textarea {
    padding: 8px 12px;
    border: 1px solid var(--acta-color-border, #e5e7eb);
    border-radius: var(--acta-radius-md, 6px);
    font-size: 14px;
    color: var(--acta-color-text, #111);
    background: var(--acta-color-bg, #fff);
    transition: border-color 0.15s;
    width: 100%;
    box-sizing: border-box;
  }

  input:focus,
  textarea:focus {
    outline: none;
    border-color: var(--acta-color-primary, #2563eb);
  }

  textarea {
    resize: vertical;
    font-family: inherit;
  }

  .cp-modal-footer {
    display: flex;
    gap: 8px;
  }

  .btn-primary,
  .btn-secondary,
  .btn-danger {
    padding: 8px 16px;
    border-radius: var(--acta-radius-md, 6px);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background 0.15s, color 0.15s;
  }

  .btn-primary {
    background: var(--acta-color-primary, #2563eb);
    color: #fff;
    border-color: var(--acta-color-primary, #2563eb);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--acta-color-primary-hover, #1d4ed8);
  }

  .btn-secondary {
    background: transparent;
    color: var(--acta-color-text, #111);
    border-color: var(--acta-color-border, #e5e7eb);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--acta-color-bg-hover, #f3f4f6);
  }

  .btn-danger {
    background: var(--acta-color-danger, #dc2626);
    color: #fff;
    border-color: var(--acta-color-danger, #dc2626);
  }

  .btn-danger:hover:not(:disabled) {
    background: var(--acta-color-danger-hover, #b91c1c);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
