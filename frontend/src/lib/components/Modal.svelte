<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';

  export let open: boolean = false;
  export let title: string;
  export let maxWidth: number = 720;

  const dispatch = createEventDispatcher<{ close: void }>();

  let dialog: HTMLDivElement | null = null;
  let previousActiveElement: HTMLElement | null = null;

  function close() {
    dispatch('close');
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      close();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }

    // Basic focus trap
    if (event.key === 'Tab' && dialog) {
      const focusable = dialog.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    }
  }

  $: if (open) {
    previousActiveElement = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    tick().then(() => {
      const firstFocusable = dialog?.querySelector<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      firstFocusable?.focus();
    });
  } else if (previousActiveElement) {
    previousActiveElement.focus();
    previousActiveElement = null;
  }
</script>

<svelte:window on:keydown={open ? handleKeydown : undefined} />

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={handleBackdropClick}>
    <div
      bind:this={dialog}
      class="modal-container"
      style:max-width="{maxWidth}px"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
    >
      <header class="modal-header">
        <h2 id="modal-title" class="modal-title">{title}</h2>
        <button class="modal-close" on:click={close} aria-label="Закрити">×</button>
      </header>
      <div class="modal-body"><slot /></div>
      {#if $$slots.footer}
        <footer class="modal-footer"><slot name="footer" /></footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--acta-color-bg-overlay);
    backdrop-filter: blur(4px);
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: backdrop-in var(--acta-motion-base);
  }

  .modal-container {
    width: 90vw;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--acta-color-bg-elevated);
    border: 1px solid var(--acta-color-border);
    border-radius: var(--acta-radius-2xl);
    box-shadow: var(--acta-shadow-modal);
    animation: modal-in var(--acta-motion-base) cubic-bezier(0.2, 0, 0, 1);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 56px;
    padding: 0 24px;
    border-bottom: 1px solid var(--acta-color-border);
    flex-shrink: 0;
  }

  .modal-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--acta-color-text);
  }

  .modal-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    color: var(--acta-color-text-muted);
    font-size: 20px;
    line-height: 1;
    border-radius: var(--acta-radius-md);
    transition: background var(--acta-motion-fast), color var(--acta-motion-fast);
  }

  .modal-close:hover {
    background: var(--acta-color-bg-hover);
    color: var(--acta-color-text);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 24px;
    border-top: 1px solid var(--acta-color-border);
    flex-shrink: 0;
  }

  @keyframes backdrop-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes modal-in {
    from { transform: scale(0.96); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }
</style>
