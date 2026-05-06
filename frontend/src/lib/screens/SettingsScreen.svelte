<script lang="ts">
  import { appShellStore } from "../stores/app-shell";
  import { settingsStore } from "../stores/settings";
  import { themeStore } from "../stores/theme";
  import type { SettingsCompanyDto, SettingsSection } from "../types";
  import AppIcon from "../components/AppIcon.svelte";
  import { importStore } from "../stores/import";

  const appShell = appShellStore;
  const settings = settingsStore;
  const theme = themeStore;

  const importBas = importStore;
  let showBasImport = false;

  const settingsSections: Array<[SettingsSection, string]> = [
    ["appearance", "Зовнішній вигляд"],
    ["company", "Компанія"],
    ["numbering", "Нумерація"],
    ["integrations", "Інтеграції"],
    ["team", "Команда"],
    ["backup", "Резервні копії"]
  ];

  function onSettingsSectionChange(section: SettingsSection) {
    settings.setSection(section);
  }

  async function onSettingsThemeChange(darkMode: boolean) {
    theme.setMode(darkMode ? "dark" : "light");
    settings.updatePreference("darkMode", darkMode);
    await settings.savePreferences();
    await appShell.reloadShellChrome();
  }

  function onSettingsCompanyFieldChange(field: keyof SettingsCompanyDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const value = input.type === "checkbox" ? input.checked : input.value;
    settings.updateCompanyField(field, value);
  }

  async function onSettingsCompanySave() {
    const result = await settings.saveCompany();
    if (result) {
      await appShell.reloadShellChrome();
    }
  }

  function integrationState(enabled: boolean) {
    return enabled
      ? { label: "Активно", tone: "is-success" }
      : { label: "Вимкнено", tone: "is-error" };
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Налаштування</h2>
      <p>Зовнішній вигляд, компанія, інтеграції, команда та резервні копії</p>
    </div>
  </div>

  <div class="settings-layout">
    <aside class="settings-nav">
      {#each settingsSections as [section, label]}
        <button
          class="btn-ghost settings-nav-button"
          class:active={$settings.section === section}
          on:click={() => onSettingsSectionChange(section)}
          disabled={$settings.loading}
        >
          {label}
        </button>
      {/each}
    </aside>

    <div class="settings-content">
      {#if $settings.message}
        <div class="status-banner is-success" role="status" aria-live="polite">
          <div>
            <strong>Зміни збережено</strong>
            <p>{$settings.message}</p>
          </div>
        </div>
      {/if}

      {#if $settings.error}
        <div class="status-banner is-error" role="alert">
          <div>
            <strong>Не вдалося виконати дію</strong>
            <p>{$settings.error}</p>
          </div>
        </div>
      {/if}

      {#if $settings.loading}
        <div class="status-banner is-loading" role="status" aria-live="polite">
          <div>
            <strong>Оновлюємо налаштування</strong>
            <p>Кнопки та поля тимчасово заблоковані, щоб уникнути дублювання дій.</p>
          </div>
        </div>
      {/if}

      {#if $settings.section === "appearance"}
        <div class="settings-card">
          <div class="settings-section-head">
            <div>
              <h3>Зовнішній вигляд</h3>
              <p>Фіксуємо канонічні стани інтерфейсу без експериментальних перемикачів.</p>
            </div>
            <span class="state-chip is-loading">Системний foundation</span>
          </div>

          <div class="segmented" data-testid="theme-segmented" role="radiogroup" aria-label="Тема інтерфейсу">
            <button
              type="button"
              role="radio"
              aria-label="Світла тема"
              aria-checked={!$settings.screen?.preferences.darkMode}
              class:active={!$settings.screen?.preferences.darkMode}
              on:click={() => onSettingsThemeChange(false)}
              disabled={$settings.loading}
            >
              Світла
            </button>
            <button
              type="button"
              role="radio"
              aria-label="Темна тема"
              aria-checked={$settings.screen?.preferences.darkMode}
              class:active={$settings.screen?.preferences.darkMode}
              on:click={() => onSettingsThemeChange(true)}
              disabled={$settings.loading}
            >
              Темна
            </button>
          </div>
        </div>
      {:else if $settings.section === "company"}
        <div class="settings-card">
          <div class="editor-header settings-section-head">
            <div>
              <h3>Компанія</h3>
              <p>{$settings.screen?.company.vatCert ?? ""}</p>
            </div>
            <div class="editor-actions">
              <button
                class="btn-primary"
                on:click={onSettingsCompanySave}
                disabled={$settings.loading}
                aria-busy={$settings.loading ? "true" : "false"}
              >
                {#if $settings.loading}
                  <span class="button-busy-label">Зберігаємо</span>
                {:else}
                  <span>Зберегти</span>
                {/if}
              </button>
            </div>
          </div>

          <div class="editor-grid cp-editor-grid">
            <label>
              Назва
              <input
                value={$settings.screen?.company.fullName ?? ""}
                on:input={(event) => onSettingsCompanyFieldChange("fullName", event)}
                disabled={$settings.loading}
              />
            </label>
            <label>
              Коротка назва
              <input
                value={$settings.screen?.company.shortName ?? ""}
                on:input={(event) => onSettingsCompanyFieldChange("shortName", event)}
                disabled={$settings.loading}
              />
            </label>
            <label>
              ЄДРПОУ
              <input
                value={$settings.screen?.company.edrpou ?? ""}
                on:input={(event) => onSettingsCompanyFieldChange("edrpou", event)}
                disabled={$settings.loading}
              />
            </label>
            <label>
              ІПН
              <input
                value={$settings.screen?.company.ipn ?? ""}
                on:input={(event) => onSettingsCompanyFieldChange("ipn", event)}
                disabled={$settings.loading}
              />
            </label>
            <label>
              IBAN
              <input
                value={$settings.screen?.company.iban ?? ""}
                on:input={(event) => onSettingsCompanyFieldChange("iban", event)}
                disabled={$settings.loading}
              />
            </label>
            <label>
              Директор
              <input
                value={$settings.screen?.company.director ?? ""}
                on:input={(event) => onSettingsCompanyFieldChange("director", event)}
                disabled={$settings.loading}
              />
            </label>
            <label class="editor-grid-span">
              Адреса
              <input
                value={$settings.screen?.company.address ?? ""}
                on:input={(event) => onSettingsCompanyFieldChange("address", event)}
                disabled={$settings.loading}
              />
            </label>
            <label class="settings-checkbox">
              <input
                type="checkbox"
                checked={$settings.screen?.company.vatRegistered ?? false}
                on:change={(event) => onSettingsCompanyFieldChange("vatRegistered", event)}
                disabled={$settings.loading}
              />
              Платник ПДВ
            </label>
          </div>
        </div>
      {:else if $settings.section === "numbering"}
        <div class="settings-card">
          <div class="settings-section-head">
            <div>
              <h3>Нумерація</h3>
              <p>Таблиця лишається читабельною та без локальних control-винятків.</p>
            </div>
          </div>

          <div class="reports-table">
            <div class="reports-table-row reports-table-head reports-table-wide settings-numbering-row">
              <span>Тип</span>
              <span>Шаблон</span>
              <span>Приклад</span>
              <span>Наступний №</span>
            </div>
            {#each $settings.screen?.numbering ?? [] as row}
              <div class="reports-table-row reports-table-wide settings-numbering-row">
                <span>{row.docType}</span>
                <span>{row.template}</span>
                <span>{row.example}</span>
                <span>{row.nextNumber}</span>
              </div>
            {/each}
          </div>
        </div>
      {:else if $settings.section === "integrations"}
        <div class="settings-card">
          <div class="settings-section-head">
            <div>
              <h3 class="title-with-icon">
                <AppIcon name="integrations" surface={true} size={18} />
                <span>Інтеграції</span>
              </h3>
              <p>Єдиний патерн для статусів, дій і проміжних станів інтеграцій.</p>
            </div>
          </div>

          <div class="linked-list">
            {#each $settings.screen?.integrations ?? [] as integration}
              <div class="settings-row">
                <div>
                  <strong>{integration.label}</strong>
                  <p>{integration.description}</p>
                </div>

                <div class="settings-row-actions">
                  <span class={`state-chip ${integrationState(integration.enabled).tone}`}>
                    {integrationState(integration.enabled).label}
                  </span>
                  <button
                    class="btn-ghost"
                    on:click={() => settings.configureIntegration(integration.tag)}
                    disabled={$settings.loading}
                    aria-busy={$settings.loading ? "true" : "false"}
                  >
                    <AppIcon name={integration.enabled ? "edit" : "add"} size={14} />
                    <span>{integration.enabled ? "Налаштувати" : "Підключити"}</span>
                  </button>

                  {#if integration.tag === "bas"}
                    <button
                      class="btn-secondary"
                      on:click={() => {
                        showBasImport = !showBasImport;
                        if (!showBasImport) importBas.reset();
                      }}
                      disabled={$settings.loading}
                    >
                      <AppIcon name="import" size={14} />
                      <span>Імпортувати</span>
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>

          {#if showBasImport}
            <div class="settings-subcard">
              {#if $importBas.error}
                <div class="status-banner is-error" role="alert">
                  <div>
                    <strong>Помилка імпорту BAS</strong>
                    <p>{$importBas.error}</p>
                  </div>
                </div>
              {/if}

              {#if $importBas.result === null}
                <div class="settings-section-head">
                  <div>
                    <h4>Імпорт BAS</h4>
                    <p>Спочатку перевіряємо експорт, а вже потім запускаємо імпорт.</p>
                  </div>
                  <span class={`state-chip ${$importBas.loading ? "is-loading" : "is-success"}`}>
                    {$importBas.loading ? "Йде перевірка" : "Готово до перевірки"}
                  </span>
                </div>

                <p>Оберіть папку, у якій лежить експорт BAS, щоб перевірити файли перед імпортом.</p>

                <div class="settings-actions-row">
                  <button
                    class="btn-secondary"
                    on:click={() => importBas.chooseDirectory()}
                    disabled={$importBas.loading || $settings.loading}
                    aria-busy={$importBas.loading ? "true" : "false"}
                  >
                    <AppIcon name="add" size={14} />
                    <span>{$importBas.selectedDirectory ? "Змінити папку" : "Обрати папку"}</span>
                  </button>
                  <button
                    class="btn-primary"
                    on:click={() => importBas.fetchPlan()}
                    disabled={$importBas.loading || $settings.loading || !$importBas.selectedDirectory}
                    aria-busy={$importBas.loading ? "true" : "false"}
                  >
                    <AppIcon name="refresh" size={14} />
                    {#if $importBas.loading}
                      <span class="button-busy-label">Перевіряємо файли</span>
                    {:else}
                      <span>Перевірити файли</span>
                    {/if}
                  </button>
                </div>

                {#if $importBas.selectedDirectory}
                  <p class="settings-inline-note">
                    Обрана папка: <code>{$importBas.selectedDirectory}</code>
                  </p>
                {/if}

                {#if $importBas.plan !== null}
                  <div class="reports-table">
                    <div class="reports-table-row reports-table-head reports-table-wide">
                      <span>Тип</span>
                      <span>Файл</span>
                      <span>Записів</span>
                      <span>Новий / Дублікат</span>
                    </div>
                    {#each $importBas.plan.entities as entity}
                      <div class="reports-table-row reports-table-wide" class:error={!!entity.error}>
                        <span>{entity.entityType}</span>
                        <span>{entity.fileName || "—"}</span>
                        <span>{entity.fileName ? entity.parsed : "—"}</span>
                        <span>
                          {#if entity.error}
                            {entity.error}
                          {:else if entity.entityType === "payments" && entity.fileName}
                            {entity.willCreate} нових / {entity.willSkip} дублікатів
                          {:else}
                            —
                          {/if}
                        </span>
                      </div>
                    {/each}
                  </div>

                  <div class="settings-actions-row">
                    <button
                      class="btn-primary"
                      on:click={() => importBas.execute()}
                      disabled={$importBas.loading || $settings.loading || ($importBas.plan?.entities.every((entity) => !entity.fileName || !!entity.error) ?? true)}
                      aria-busy={$importBas.loading ? "true" : "false"}
                    >
                      <AppIcon name="save" size={14} />
                      {#if $importBas.loading}
                        <span class="button-busy-label">Виконуємо імпорт</span>
                      {:else}
                        <span>Виконати імпорт</span>
                      {/if}
                    </button>
                    <button
                      class="btn-ghost"
                      on:click={() => {
                        showBasImport = false;
                        importBas.reset();
                      }}
                      disabled={$importBas.loading || $settings.loading}
                    >
                      <span>Скасувати</span>
                    </button>
                  </div>
                {/if}
              {:else}
                <div class="settings-section-head">
                  <div>
                    <h4>Результат імпорту</h4>
                    <p>Підсумок по кожному типу сутностей після виконання.</p>
                  </div>
                  <span class="state-chip is-success">Імпорт завершено</span>
                </div>

                <div class="reports-table">
                  <div class="reports-table-row reports-table-head reports-table-wide">
                    <span>Тип</span>
                    <span>Створено</span>
                    <span>Оновлено</span>
                    <span>Пропущено</span>
                    <span>Конфлікти</span>
                  </div>
                  {#each $importBas.result.entities as entity}
                    <div class="reports-table-row reports-table-wide" class:error={!!entity.error}>
                      <span>{entity.entityType}</span>
                      <span>{entity.created}</span>
                      <span>{entity.updated}</span>
                      <span>{entity.skipped}</span>
                      <span>{entity.conflicts}</span>
                    </div>
                  {/each}
                </div>

                {#each $importBas.result.entities.filter((entity) => entity.error) as entity}
                  <p class="error">{entity.entityType}: {entity.error}</p>
                {/each}

                <div class="settings-actions-row">
                  <button
                    class="btn-ghost"
                    on:click={() => {
                      showBasImport = false;
                      importBas.reset();
                    }}
                    disabled={$settings.loading}
                  >
                    <span>Закрити</span>
                  </button>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {:else if $settings.section === "team"}
        <div class="settings-card">
          <div class="editor-header settings-section-head">
            <div>
              <h3>Команда</h3>
              <p>{$settings.screen?.team.length ?? 0} користувачів</p>
            </div>
            <div class="editor-actions">
              <button
                class="btn-secondary"
                on:click={() => settings.inviteTeam()}
                disabled={$settings.loading}
                aria-busy={$settings.loading ? "true" : "false"}
              >
                {#if $settings.loading}
                  <span class="button-busy-label">Надсилаємо запрошення</span>
                {:else}
                  <span>Запросити</span>
                {/if}
              </button>
            </div>
          </div>

          <div class="linked-list">
            {#each $settings.screen?.team ?? [] as member}
              <div class="settings-row">
                <div>
                  <strong>{member.name}</strong>
                  <p>{member.email}</p>
                </div>
                <div class="settings-row-actions">
                  <span class="state-chip">{member.role}</span>
                  <span class="settings-meta">{member.lastActive}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="settings-card">
          <div class="editor-header settings-section-head">
            <div>
              <h3>Резервні копії</h3>
              <p>{$settings.screen?.backup.kind ?? ""}</p>
            </div>
            <div class="editor-actions">
              <button
                class="btn-secondary"
                on:click={() => settings.openLatestBackup()}
                disabled={$settings.loading}
                aria-busy={$settings.loading ? "true" : "false"}
              >
                <span>Відкрити копію</span>
              </button>
              <button
                class="btn-secondary"
                on:click={() => settings.backupNow()}
                disabled={$settings.loading}
                aria-busy={$settings.loading ? "true" : "false"}
              >
                {#if $settings.loading}
                  <span class="button-busy-label">Створюємо копію</span>
                {:else}
                  <span>Створити зараз</span>
                {/if}
              </button>
            </div>
          </div>

          <div class="task-kpi-card">
            <strong>{$settings.screen?.backup.label ?? "-"}</strong>
            <span>{$settings.screen?.backup.file ?? ""}</span>
            <span>{$settings.screen?.backup.note ?? ""}</span>
          </div>
        </div>
      {/if}
    </div>
  </div>
</section>
