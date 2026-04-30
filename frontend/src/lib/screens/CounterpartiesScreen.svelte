<script lang="ts">
  import { counterpartiesStore } from "../stores/counterparties";
  import { documentsStore } from "../stores/documents";
  import { navigationStore } from "../stores/navigation";

  const counterparties = counterpartiesStore;
  const documents = documentsStore;
  const navigation = navigationStore;

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
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Контрагенти</h2>
      <p>{$counterparties.screen?.items.length ?? 0} записів</p>
    </div>
    <div class="panel-actions">
      <input placeholder="Пошук контрагентів" on:input={onCounterpartySearch} />
      <button on:click={() => counterparties.openEditor()}>Новий контрагент</button>
    </div>
  </div>

  {#if $counterparties.message}
    <p class="message">{$counterparties.message}</p>
  {/if}

  {#if $counterparties.error}
    <p class="error">{$counterparties.error}</p>
  {/if}

  <div class="counterparties-layout">
    <div class="counterparties-list">
      {#each $counterparties.screen?.items ?? [] as item}
        <button
          class:active={$counterparties.selectedId === item.id}
          class="counterparty-row"
          on:click={() => counterparties.open(item.id)}
        >
          <div>
            <strong>{item.name}</strong>
            <p>{item.edrpou || "Без ЄДРПОУ"}</p>
          </div>
        </button>
      {/each}
    </div>

    <div class="counterparty-detail">
      {#if $counterparties.detail}
        <div class="counterparty-detail-header">
          <div>
            <h3>{$counterparties.detail.info.name}</h3>
            <p>{$counterparties.detail.info.edrpou || "Без ЄДРПОУ"}</p>
          </div>
          <div class="editor-actions">
            <button on:click={() => counterparties.openEditor($counterparties.detail?.info.id)}>
              Редагувати
            </button>
            <button on:click={() => counterparties.createDocument()}>Створити документ</button>
            <button class="ghost-danger" on:click={() => counterparties.archiveCurrent()}>
              Архівувати
            </button>
          </div>
        </div>

        <div class="detail-grid">
          <div>
            <strong>IBAN</strong>
            <p>{$counterparties.detail.info.iban || "—"}</p>
          </div>
          <div>
            <strong>Телефон</strong>
            <p>{$counterparties.detail.info.phone || "—"}</p>
          </div>
          <div>
            <strong>Email</strong>
            <p>{$counterparties.detail.info.email || "—"}</p>
          </div>
          <div>
            <strong>Адреса</strong>
            <p>{$counterparties.detail.info.address || "—"}</p>
          </div>
        </div>

        <div class="linked-block">
          <strong>Документи</strong>
          <div class="linked-list">
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
          </div>
        </div>

        <div class="linked-block">
          <strong>Платежі</strong>
          <div class="linked-list">
            {#each $counterparties.detail.payments as payment}
              <div class="linked-row static">
                <span>{payment.date}</span>
                <span>{payment.amountStr}</span>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="empty-screen compact">
          <p>Виберіть контрагента зі списку.</p>
        </div>
      {/if}
    </div>
  </div>
</section>

{#if $counterparties.editor}
  <section class="editor-sheet">
    <div class="editor-header">
      <div>
        <h3>{$counterparties.editor.form.title}</h3>
        <p>Картка контрагента</p>
      </div>
      <div class="editor-actions">
        <button on:click={() => counterparties.save()}>Зберегти</button>
        <button on:click={() => counterparties.closeEditor()}>Закрити</button>
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
