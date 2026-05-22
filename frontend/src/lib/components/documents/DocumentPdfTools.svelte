<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { DocumentEditorDto } from "../../types";

  export let documentId = "";
  export let pdf: DocumentEditorDto["pdf"] = null;
  export let loading = false;

  const dispatch = createEventDispatcher<{
    attachExistingPdf: void;
    openCurrentPdf: void;
    applyTextReplace: { findText: string; replaceText: string };
  }>();

  let pdfFindText = "";
  let pdfReplaceText = "";
  let lastDocumentId = "";

  $: if (documentId !== lastDocumentId) {
    lastDocumentId = documentId;
    pdfFindText = "";
    pdfReplaceText = "";
  }

  function applyTextReplace() {
    dispatch("applyTextReplace", {
      findText: pdfFindText,
      replaceText: pdfReplaceText
    });
  }
</script>

<div class="editor-items-card existing-pdf-card" data-testid="documents-existing-pdf">
  <div class="editor-items-header">
    <div>
      <strong>Існуючий PDF</strong>
      {#if pdf}
        <p class="existing-pdf-status">
          {pdf.filePath} · {pdf.pageCount} стор. ·
          {pdf.editable ? "Exact replace доступний" : "Тільки перегляд"}
        </p>
      {:else}
        <p class="existing-pdf-status">Не прив'язано</p>
      {/if}
    </div>
    <div class="editor-actions existing-pdf-actions">
      <button class="btn-secondary" on:click={() => dispatch("attachExistingPdf")} disabled={loading}>
        {pdf ? "Прив'язати інший PDF" : "Прив'язати PDF"}
      </button>
      {#if pdf}
        <button
          class="btn-ghost"
          on:click={() => dispatch("openCurrentPdf")}
          disabled={loading}
        >
          Відкрити PDF
        </button>
      {/if}
    </div>
  </div>

  {#if pdf}
    <details class="existing-pdf-details" open={pdf.warnings.length > 0}>
      <summary>Текстовий шар і exact replace</summary>

      <p class="existing-pdf-meta">
        Текстовий шар: {pdf.hasTextOps ? "Знайдено" : "Не знайдено"}
      </p>

      {#if pdf.warnings.length > 0}
        <div class="existing-pdf-warnings">
          {#each pdf.warnings as warning}
            <p>{warning}</p>
          {/each}
        </div>
      {/if}

      <label class="existing-pdf-preview">
        Витягнутий текст
        <textarea rows="10" readonly value={pdf.extractedText}></textarea>
      </label>

      <div class="existing-pdf-replace">
        <label>
          <span>Знайти текст</span>
          <input bind:value={pdfFindText} placeholder="Точний фрагмент з витягнутого тексту" />
        </label>
        <label>
          <span>Замінити на</span>
          <input bind:value={pdfReplaceText} placeholder="Новий текст" />
        </label>
        <button
          class="btn-primary"
          on:click={applyTextReplace}
          disabled={
            loading ||
            !pdf.editable ||
            !pdfFindText.trim() ||
            !pdfReplaceText.trim()
          }
        >
          Застосувати exact replace
        </button>
      </div>
    </details>
  {/if}
</div>
