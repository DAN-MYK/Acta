<script lang="ts">
  import AppIcon from "../AppIcon.svelte";
  import {
    DOCUMENT_KIND_META,
    DOCUMENT_KIND_OPTIONS,
    getDocumentCreateLabel
  } from "../../config/ui";
  import type { DocumentDirection, DocumentKind } from "../../types";

  export let open = false;
  export let loading = false;
  export let disabled = false;
  export let selectedKind: DocumentKind | null = null;
  export let activeTab: "all" | DocumentDirection = "all";

  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher<{
    toggle: void;
    directCreate: DocumentKind;
    menuCreate: DocumentKind;
  }>();

  $: buttonKind = selectedKind ?? null;
  $: buttonLabel = buttonKind
    ? getDocumentCreateLabel(buttonKind, activeTab)
    : "Створити ▾";

  function onCreateClick() {
    if (buttonKind) {
      dispatch("directCreate", buttonKind);
      return;
    }

    dispatch("toggle");
  }
</script>

<button
  class="btn-primary"
  data-testid="documents-create-button"
  type="button"
  disabled={loading || disabled}
  on:click={onCreateClick}
  aria-expanded={open}
  aria-controls={open ? "documents-create-picker" : undefined}
  aria-busy={loading ? "true" : "false"}
>
  {#if buttonKind}
    <AppIcon name={DOCUMENT_KIND_META[buttonKind].icon} surface={true} />
  {/if}
  <span>{buttonLabel}</span>
</button>

{#if open}
  <div
    id="documents-create-picker"
    class="create-picker-popover"
    data-testid="documents-create-picker"
    role="menu"
    aria-label="Створити документ"
  >
    {#each DOCUMENT_KIND_OPTIONS as option}
      <button
        type="button"
        class="create-picker-item"
        data-testid={`documents-create-picker-${option.value}`}
        role="menuitem"
        on:click={() => dispatch("menuCreate", option.value)}
      >
        <AppIcon name={DOCUMENT_KIND_META[option.value].icon} surface={true} />
        <span>{option.label}</span>
      </button>
    {/each}
  </div>
{/if}
