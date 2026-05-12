<script lang="ts">
  export let label: string;
  export let id: string | undefined = undefined;
  export let required: boolean = false;
  export let error: string | undefined = undefined;
  export let helpText: string | undefined = undefined;
</script>

<div class="field">
  <label class="label" for={id}>
    {label}{#if required}<span class="required" aria-hidden="true"> *</span>{/if}
  </label>
  <slot />
  {#if error}
    <p class="error-text" role="alert"><span aria-hidden="true">⚠</span> {error}</p>
  {:else if helpText}
    <p class="help-text">{helpText}</p>
  {/if}
</div>

<style>
  .field {
    display: grid;
    gap: 6px;
  }

  .label {
    font-size: 13px;
    font-weight: 500;
    color: var(--acta-color-text-muted);
  }

  .required {
    color: var(--acta-color-danger);
  }

  .error-text {
    margin: 0;
    font-size: 12px;
    color: var(--acta-color-danger);
  }

  .help-text {
    margin: 0;
    font-size: 12px;
    color: var(--acta-color-text-faint);
  }
</style>
