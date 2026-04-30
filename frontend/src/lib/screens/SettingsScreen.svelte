<script lang="ts">
  import { settingsStore } from "../stores/settings";
  import { shellStore } from "../stores/shell";
  import { themeStore } from "../stores/theme";
  import type { SettingsCompanyDto, SettingsSection } from "../types";
  import AppIcon from "../components/AppIcon.svelte";
  import { importStore } from "../stores/import";

  const settings = settingsStore;
  const shell = shellStore;
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
    await shell.load();
  }

  async function onSettingsDensityChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    settings.updatePreference("density", Number(select.value));
    await settings.savePreferences();
  }

  function onSettingsCompanyFieldChange(field: keyof SettingsCompanyDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const value = input.type === "checkbox" ? input.checked : input.value;
    settings.updateCompanyField(field, value);
  }

  async function onSettingsCompanySave() {
    const result = await settings.saveCompany();
    if (result) {
      await shell.load();
    }
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>Налаштування</h2>
      <p>Tauri vertical slice для appearance, company, integrations, team та backup</p>
    </div>
  </div>

  <div class="settings-layout">
    <aside class="settings-nav">
      {#each settingsSections as [section, label]}
        <button class:active={$settings.section === section} on:click={() => onSettingsSectionChange(section)}>
          {label}
        </button>
      {/each}
    </aside>

    <div class="settings-content">
      {#if $settings.message}
        <p class="message">{$settings.message}</p>
      {/if}

      {#if $settings.error}
        <p class="error">{$settings.error}</p>
      {/if}

      {#if $settings.section === "appearance"}
        <div class="settings-card">
          <h3>Зовнішній вигляд</h3>
          <div class="settings-actions-row">
            <button class:active={!$settings.screen?.preferences.darkMode} on:click={() => onSettingsThemeChange(false)}>
              Світла тема
            </button>
            <button class:active={$settings.screen?.preferences.darkMode} on:click={() => onSettingsThemeChange(true)}>
              Темна тема
            </button>
            <select value={$settings.screen?.preferences.density ?? 1} on:change={onSettingsDensityChange}>
              <option value="0">Compact</option>
              <option value="1">Comfortable</option>
              <option value="2">Spacious</option>
            </select>
          </div>
        </div>
      {:else if $settings.section === "company"}
        <div class="settings-card">
          <div class="editor-header">
            <div>
              <h3>Компанія</h3>
              <p>{$settings.screen?.company.vatCert ?? ""}</p>
            </div>
            <div class="editor-actions">
              <button on:click={onSettingsCompanySave}>Зберегти</button>
            </div>
          </div>

          <div class="editor-grid cp-editor-grid">
            <label>
              Назва
              <input value={$settings.screen?.company.fullName ?? ""} on:input={(event) => onSettingsCompanyFieldChange("fullName", event)} />
            </label>
            <label>
              Коротка назва
              <input value={$settings.screen?.company.shortName ?? ""} on:input={(event) => onSettingsCompanyFieldChange("shortName", event)} />
            </label>
            <label>
              ЄДРПОУ
              <input value={$settings.screen?.company.edrpou ?? ""} on:input={(event) => onSettingsCompanyFieldChange("edrpou", event)} />
            </label>
            <label>
              ІПН
              <input value={$settings.screen?.company.ipn ?? ""} on:input={(event) => onSettingsCompanyFieldChange("ipn", event)} />
            </label>
            <label>
              IBAN
              <input value={$settings.screen?.company.iban ?? ""} on:input={(event) => onSettingsCompanyFieldChange("iban", event)} />
            </label>
            <label>
              Директор
              <input value={$settings.screen?.company.director ?? ""} on:input={(event) => onSettingsCompanyFieldChange("director", event)} />
            </label>
            <label class="editor-grid-span">
              Адреса
              <input value={$settings.screen?.company.address ?? ""} on:input={(event) => onSettingsCompanyFieldChange("address", event)} />
            </label>
            <label class="settings-checkbox">
              <input
                type="checkbox"
                checked={$settings.screen?.company.vatRegistered ?? false}
                on:change={(event) => onSettingsCompanyFieldChange("vatRegistered", event)}
              />
              Платник ПДВ
            </label>
          </div>
        </div>
      {:else if $settings.section === "numbering"}
        <div class="settings-card">
          <h3>Нумерація</h3>
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
          <h3 class="title-with-icon"><AppIcon name="integrations" surface={true} size={18} /><span>Інтеграції</span></h3>
          <div class="linked-list">
            {#each $settings.screen?.integrations ?? [] as integration}
              <div class="settings-row">
                <div>
                  <strong>{integration.label}</strong>
                  <p>{integration.description}</p>
                </div>
                <div class="settings-row-actions">
                  <span>{integration.enabled ? "Активно" : "Вимкнено"}</span>
                  <button class="action-button compact" on:click={() => settings.configureIntegration(integration.tag)}>
                    <AppIcon name={integration.enabled ? "edit" : "add"} size={14} />
                    <span>{integration.enabled ? "Налаштувати" : "Підключити"}</span>
                  </button>
                  {#if integration.tag === "bas"}
                    <button
                      class="action-button compact"
                      on:click={() => { showBasImport = !showBasImport; if (!showBasImport) importBas.reset(); }}
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
            <div class="settings-card" style="margin-top: 1rem;">
              {#if $importBas.error}
                <p class="error">{$importBas.error}</p>
              {/if}

              {#if $importBas.result === null}
                <p>Помістіть файли BAS у <code>storage/import/bas/</code></p>
                <div class="settings-actions-row" style="margin-top: 0.5rem;">
                  <button
                    class="action-button compact"
                    on:click={() => importBas.plan()}
                    disabled={$importBas.loading}
                  >
                    <AppIcon name="refresh" size={14} />
                    <span>{$importBas.loading ? "Перевірка..." : "Перевірити файли"}</span>
                  </button>
                </div>

                {#if $importBas.plan !== null}
                  <div class="reports-table" style="margin-top: 1rem;">
                    <div class="reports-table-row reports-table-head reports-table-wide">
                      <span>Тип</span><span>Файл</span><span>Записів</span><span>Новий / Дублікат</span>
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
                  <div class="settings-actions-row" style="margin-top: 0.5rem;">
                    <button
                      class="action-button compact"
                      on:click={() => importBas.execute()}
                      disabled={$importBas.loading || $importBas.plan!.entities.every(e => !e.fileName || !!e.error)}
                    >
                      <AppIcon name="save" size={14} />
                      <span>{$importBas.loading ? "Виконання..." : "Виконати імпорт"}</span>
                    </button>
                    <button
                      class="action-button compact"
                      on:click={() => { showBasImport = false; importBas.reset(); }}
                    >
                      <span>Скасувати</span>
                    </button>
                  </div>
                {/if}
              {:else}
                <div class="reports-table">
                  <div class="reports-table-row reports-table-head reports-table-wide">
                    <span>Тип</span><span>Створено</span><span>Оновлено</span><span>Пропущено</span><span>Конфлікти</span>
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
                {#each $importBas.result.entities.filter((e) => e.error) as entity}
                  <p class="error">{entity.entityType}: {entity.error}</p>
                {/each}
                <div class="settings-actions-row" style="margin-top: 0.5rem;">
                  <button
                    class="action-button compact"
                    on:click={() => { showBasImport = false; importBas.reset(); }}
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
          <div class="editor-header">
            <div>
              <h3>Команда</h3>
              <p>{$settings.screen?.team.length ?? 0} користувачів</p>
            </div>
            <div class="editor-actions">
              <button on:click={() => settings.inviteTeam()}>Запросити</button>
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
                  <span>{member.role}</span>
                  <span>{member.lastActive}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="settings-card">
          <div class="editor-header">
            <div>
              <h3>Резервні копії</h3>
              <p>{$settings.screen?.backup.kind ?? ""}</p>
            </div>
            <div class="editor-actions">
              <button on:click={() => settings.openLatestBackup()}>Відкрити копію</button>
              <button on:click={() => settings.backupNow()}>Створити зараз</button>
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
