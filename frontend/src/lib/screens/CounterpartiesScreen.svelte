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

  function getRiskLabel(overdueCount: number): string {
    if (overdueCount <= 0) {
      return "Працює стабільно";
    }

    return `Потребує уваги: прострочено ${overdueCount} ${
      overdueCount === 1 ? "документ" : "документи"
    }`;
  }

  function getLastContactLabel(days: number): string {
    if (days === 1) {
      return "1 день тому";
    }

    if (days >= 2 && days <= 4) {
      return `${days} дні тому`;
    }

    return `${days} днів тому`;
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
    <p class="message">Оновлюємо картку контрагента…</p>
  {/if}

  <div class="counterparties-layout">
    <div class="counterparties-list">
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
            <span class="task-pill">{item.kind}</span>
            <span>{item.balanceStr}</span>
            {#if item.overdueCount > 0}
              <span class="risk-chip risk-chip-danger">Прострочка {item.overdueCount}</span>
            {/if}
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
            <div class="counterparty-overview-badges">
              <span class="task-pill">{$counterparties.detail.info.kind}</span>
              <span
                class:risk-chip-danger={$counterparties.detail.info.overdueCount > 0}
                class:risk-chip-ok={$counterparties.detail.info.overdueCount <= 0}
                class="risk-chip"
              >
                {getRiskLabel($counterparties.detail.info.overdueCount)}
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
            <button class="btn-danger" on:click={() => counterparties.archiveCurrent()}>
              Архівувати
            </button>
          </div>
        </div>

        <div class="chain-panel counterparty-overview-panel">
          <div class="chain-panel-header">
            <div>
              <strong>Фінансовий стан</strong>
              <p>Картка показує не лише реквізити, а й ризики, активність та найближчу дію по контрагенту.</p>
            </div>
            <div class="chain-summary counterparty-overview">
              <div class="chain-summary-block">
                <span>Баланс</span>
                <strong>{$counterparties.detail.info.balanceStr}</strong>
              </div>
              <div class="chain-summary-block">
                <span>Наступна дія</span>
                <strong>{getRiskLabel($counterparties.detail.info.overdueCount)}</strong>
              </div>
              <div class="chain-summary-block">
                <span>Прострочка</span>
                <strong>{$counterparties.detail.info.overdueAmountStr}</strong>
              </div>
              <div class="chain-summary-block">
                <span>Останній контакт</span>
                <strong>{$counterparties.detail.info.lastContactDate}</strong>
              </div>
            </div>
          </div>
        </div>

        <div class="detail-grid">
          <div>
            <strong>Прострочка</strong>
            <p>{getRiskLabel($counterparties.detail.info.overdueCount)}</p>
          </div>
          <div>
            <strong>Сума прострочки</strong>
            <p>{$counterparties.detail.info.overdueAmountStr}</p>
          </div>
          <div>
            <strong>Останній контакт</strong>
            <p>{$counterparties.detail.info.lastContactDate} • {getLastContactLabel($counterparties.detail.info.lastContactDays)}</p>
          </div>
          <div>
            <strong>Документів</strong>
            <p>{$counterparties.detail.info.docCount}</p>
          </div>
          <div>
            <strong>Директор</strong>
            <p>{$counterparties.detail.info.director || "—"}</p>
          </div>
          <div>
            <strong>Банк</strong>
            <p>{$counterparties.detail.info.bank || "—"}</p>
          </div>
          <div>
            <strong>VAT</strong>
            <p>{$counterparties.detail.info.vat || "—"}</p>
          </div>
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
          <div class="editor-grid-span">
            <strong>Адреса</strong>
            <p>{$counterparties.detail.info.address || "—"}</p>
          </div>
        </div>

        <div class="linked-block">
          <strong>Документи</strong>
          <p>Відкрийте документи контрагента, щоб швидко перейти до боргу або наступного статусу.</p>
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
          <p>Оцініть останні рухи коштів і зв'язок із документами без переходу на інший екран.</p>
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
        <div class="empty-screen empty-state-card compact">
          <strong>Оберіть контрагента</strong>
          <p>Побачите баланс, прострочки, останній контакт і пов'язані документи без переходів між екранами.</p>
          <button class="btn-primary" on:click={() => counterparties.openEditor()}>Новий контрагент</button>
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
        <button class="btn-primary" on:click={() => counterparties.save()}>Зберегти</button>
        <button class="btn-ghost" on:click={() => counterparties.closeEditor()}>Закрити</button>
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
