<script lang="ts">
  import { onMount, tick } from "svelte";
  import { counterpartiesStore } from "./lib/stores/counterparties";
  import { dashboardStore } from "./lib/stores/dashboard";
  import { documentsStore } from "./lib/stores/documents";
  import { navigationStore } from "./lib/stores/navigation";
  import { paletteStore } from "./lib/stores/palette";
  import { reportsStore } from "./lib/stores/reports";
  import { settingsStore } from "./lib/stores/settings";
  import { shellStore } from "./lib/stores/shell";
  import { tasksStore } from "./lib/stores/tasks";
  import { paymentsStore } from "./lib/stores/payments";
  import { themeStore } from "./lib/stores/theme";
  import CounterpartiesScreen from "./lib/screens/CounterpartiesScreen.svelte";
  import DashboardScreen from "./lib/screens/DashboardScreen.svelte";
  import DocumentsScreen from "./lib/screens/DocumentsScreen.svelte";
  import PaymentsScreen from "./lib/screens/PaymentsScreen.svelte";
  import ReportsScreen from "./lib/screens/ReportsScreen.svelte";
  import SettingsScreen from "./lib/screens/SettingsScreen.svelte";
  import AppIcon from "./lib/components/AppIcon.svelte";
  import type {
    ScreenId,
    TaskDraftFormDto,
    TaskItemDto,
    TaskStatus
  } from "./lib/types";

  const navigation = navigationStore;
  const shell = shellStore;
  const palette = paletteStore;
  const theme = themeStore;
  const dashboard = dashboardStore;
  const documents = documentsStore;
  const counterparties = counterpartiesStore;
  const tasks = tasksStore;
  const reports = reportsStore;
  const payments = paymentsStore;
  const settings = settingsStore;

  let paletteInput: HTMLInputElement | null = null;

  const sidebarScreens: Array<{
    screen: ScreenId;
    label: string;
    icon: "dashboard" | "documents" | "counterparties" | "payments" | "reports" | "tasks" | "settings";
  }> = [
    { screen: "dashboard", label: "Дашборд", icon: "dashboard" },
    { screen: "documents", label: "Документи", icon: "documents" },
    { screen: "counterparties", label: "Контрагенти", icon: "counterparties" },
    { screen: "payments", label: "Платежі", icon: "payments" },
    { screen: "reports", label: "Звіти", icon: "reports" },
    { screen: "tasks", label: "Завдання", icon: "tasks" },
    { screen: "settings", label: "Налаштування", icon: "settings" }
  ];

  onMount(async () => {
    await shell.load();
    const settingsScreen = await settings.load();
    if (settingsScreen) {
      theme.setMode(settingsScreen.preferences.darkMode ? "dark" : "light");
    }
    await Promise.all([
      dashboard.load(),
      documents.load(),
      counterparties.load(),
      tasks.load(),
      reports.load(),
      payments.load()
    ]);
  });

  $: if ($palette.open) {
    void tick().then(() => paletteInput?.focus());
  }

  $: document.body.dataset.theme = $theme;
  $: currentScreen = $navigation;
  $: shellState = $shell.state;

  function onPaletteInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void palette.search(input.value);
  }

  function onTaskSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void tasks.load(input.value);
  }

  async function onCompanyChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    await shell.setActiveCompany(select.value);
    const settingsScreen = await settings.load();
    if (settingsScreen) {
      theme.setMode(settingsScreen.preferences.darkMode ? "dark" : "light");
    }
    await Promise.all([
      dashboard.load(),
      documents.load(),
      counterparties.load(),
      tasks.load(),
      reports.load(),
      payments.load()
    ]);
  }

  async function onQuickThemeToggle() {
    const darkMode = $theme === "light";
    theme.setMode(darkMode ? "dark" : "light");
    settings.updatePreference("darkMode", darkMode);
    await settings.savePreferences();
    await shell.load();
  }

  function onTaskFieldChange(field: keyof TaskDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    tasks.updateFormField(field, input.value);
  }

  function taskItemsForTab(items: TaskItemDto[], tab: "open" | "done" | "all") {
    if (tab === "done") {
      return items.filter((item) => item.status === "done" || item.status === "cancelled");
    }
    if (tab === "all") {
      return items;
    }
    return items.filter((item) => item.status === "open" || item.status === "in_progress");
  }

  function todayTaskItems(items: TaskItemDto[]) {
    const today = new Date().toISOString().slice(0, 10);
    return items.filter((item) => item.dueDate === today || item.reminderAt.startsWith(today));
  }

  function toggleTaskStatus(task: TaskItemDto) {
    const nextStatus: TaskStatus = task.status === "done" ? "open" : "done";
    void tasks.setStatus(task.id, nextStatus);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key.toLowerCase() === "k") {
      event.preventDefault();
      palette.toggle();
    }

    if (event.ctrlKey && event.key >= "1" && event.key <= "7") {
      event.preventDefault();
      const screens: ScreenId[] = [
        "dashboard",
        "documents",
        "counterparties",
        "payments",
        "reports",
        "tasks",
        "settings"
      ];
      navigation.go(screens[Number(event.key) - 1]);
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">A</div>
      <div>
        <strong>Acta</strong>
        <p>Tauri migration scaffold</p>
      </div>
    </div>

    <nav class="nav">
      {#each sidebarScreens as item}
        <button class:active={currentScreen === item.screen} on:click={() => navigation.go(item.screen)}>
          <AppIcon name={item.icon} surface={currentScreen === item.screen} />
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>

    <div class="theme-switcher">
      <button on:click={() => theme.toggle()}>
        <AppIcon name="theme" surface={true} />
        <span>Тема: {$theme}</span>
      </button>
    </div>
  </aside>

  <main class="content">
    <header class="topbar">
      <div>
        <h1>{shellState?.chrome.companyName ?? "Acta"}</h1>
        <p>{shellState?.chrome.userRole ?? "Завантаження shell..."}</p>
      </div>

      <div class="topbar-actions">
        <select value={shellState?.activeCompanyId} on:change={onCompanyChange}>
          {#each shellState?.companyItems ?? [] as company}
            <option value={company.id}>{company.name}</option>
          {/each}
        </select>

        <button on:click={() => palette.toggle()}>
          <AppIcon name="palette" surface={true} />
          <span>Ctrl+K</span>
        </button>
      </div>
    </header>

    {#if currentScreen === "dashboard"}
      <DashboardScreen />
    {:else if currentScreen === "documents"}
      <DocumentsScreen />
    {:else if currentScreen === "counterparties"}
      <CounterpartiesScreen />
    {:else if currentScreen === "reports"}
      <ReportsScreen />
    {:else if currentScreen === "tasks"}
      <section class="panel">
        <div class="panel-header">
          <div>
            <h2>Завдання</h2>
            <p>{$tasks.screen?.items.length ?? 0} записів у поточній вибірці</p>
          </div>
          <div class="panel-actions">
            <input placeholder="Пошук завдань" on:input={onTaskSearch} />
            <button on:click={() => tasks.openEditor()}>Нове завдання</button>
          </div>
        </div>

        <div class="task-kpis">
          <div class="task-kpi-card">
            <strong>{$tasks.screen?.openCount ?? 0}</strong>
            <span>Активні</span>
          </div>
          <div class="task-kpi-card">
            <strong>{$tasks.screen?.doneCount ?? 0}</strong>
            <span>Завершені</span>
          </div>
          <div class="task-kpi-card">
            <strong>{$tasks.screen?.highCount ?? 0}</strong>
            <span>Високий пріоритет</span>
          </div>
          <div class="task-kpi-card">
            <strong>{$tasks.screen?.todayCount ?? 0}</strong>
            <span>На сьогодні</span>
          </div>
        </div>

        {#if $tasks.message}
          <p class="message">{$tasks.message}</p>
        {/if}

        {#if $tasks.error}
          <p class="error">{$tasks.error}</p>
        {/if}

        <div class="tasks-layout">
          <div class="tasks-main">
            <div class="task-tabs">
              <button class:active={$tasks.tab === "open"} on:click={() => tasks.setTab("open")}>Активні</button>
              <button class:active={$tasks.tab === "done"} on:click={() => tasks.setTab("done")}>Завершені</button>
              <button class:active={$tasks.tab === "all"} on:click={() => tasks.setTab("all")}>Усі</button>
            </div>

            <div class="tasks-list">
              {#each taskItemsForTab($tasks.screen?.items ?? [], $tasks.tab) as item}
                <div class="task-row">
                  <button class="task-row-main" on:click={() => tasks.openEditor(item.id)}>
                    <div>
                      <strong>{item.title}</strong>
                      <p>{item.description || item.priorityLabel}</p>
                    </div>
                    <div class="task-row-meta">
                      <span class="task-pill">{item.priorityLabel}</span>
                      <span>{item.dueDate || "Без дедлайну"}</span>
                      <span>{item.statusLabel}</span>
                    </div>
                  </button>
                  <button on:click={() => toggleTaskStatus(item)}>
                    {item.status === "done" ? "Повернути" : "Готово"}
                  </button>
                </div>
              {/each}
            </div>
          </div>

          <aside class="tasks-side-panel">
            <strong>Сьогодні</strong>
            <div class="linked-list">
              {#each todayTaskItems($tasks.screen?.items ?? []) as item}
                <button class="linked-row" on:click={() => tasks.openEditor(item.id)}>
                  <span>{item.title}</span>
                  <span>{item.reminderAt || item.dueDate}</span>
                </button>
              {/each}
            </div>
          </aside>
        </div>
      </section>
    {:else if currentScreen === "settings"}
      <SettingsScreen />
    {:else if currentScreen === "payments"}
      <PaymentsScreen />
    {:else}
      <section class="panel empty-screen">
        <h2>{currentScreen}</h2>
        <p>Для першого vertical slice зараз реалізовано shell, documents, counterparties, tasks та reports.</p>
      </section>
    {/if}

    {#if currentScreen === "tasks" && $tasks.editor}
      <section class="editor-sheet">
        <div class="editor-header">
          <div>
            <h3>{$tasks.editor.title}</h3>
            <p>{$tasks.editor.form.linkLabel || "Без прив'язки"}</p>
          </div>
          <div class="editor-actions">
            <button on:click={() => tasks.save()}>Зберегти</button>
            {#if $tasks.editor.form.id}
              <button class="ghost-danger" on:click={() => tasks.deleteCurrent()}>Видалити</button>
            {/if}
            <button on:click={() => tasks.closeEditor()}>Закрити</button>
          </div>
        </div>

        <div class="editor-grid">
          <label class="editor-grid-span">
            Назва
            <input value={$tasks.editor.form.title} on:input={(event) => onTaskFieldChange("title", event)} />
          </label>
          <label class="editor-grid-span">
            Опис
            <textarea rows="4" value={$tasks.editor.form.description} on:input={(event) => onTaskFieldChange("description", event)}></textarea>
          </label>
          <label>
            Пріоритет
            <select value={$tasks.editor.form.priority} on:change={(event) => onTaskFieldChange("priority", event)}>
              <option value="low">Низький</option>
              <option value="normal">Звичайний</option>
              <option value="high">Високий</option>
              <option value="critical">Критичний</option>
            </select>
          </label>
          <label>
            Статус
            <select value={$tasks.editor.form.status} on:change={(event) => onTaskFieldChange("status", event)}>
              <option value="open">Відкрите</option>
              <option value="in_progress">В роботі</option>
              <option value="done">Виконано</option>
              <option value="cancelled">Скасовано</option>
            </select>
          </label>
          <label>
            Дедлайн
            <input type="date" value={$tasks.editor.form.dueDate} on:input={(event) => onTaskFieldChange("dueDate", event)} />
          </label>
          <label>
            Нагадування
            <input type="datetime-local" value={$tasks.editor.form.reminderAt} on:input={(event) => onTaskFieldChange("reminderAt", event)} />
          </label>
        </div>
      </section>
    {/if}

  </main>

  {#if $palette.open}
    <button type="button" class="palette-backdrop" aria-label="Закрити палітру команд" on:click={() => palette.close()}></button>
    <section class="palette">
      <input bind:this={paletteInput} placeholder="Пошук команд, екранів і документів" on:input={onPaletteInput} />

      <div class="palette-items">
        {#each $palette.items as item}
          <button
            class="palette-item"
            on:click={async () => {
              await palette.activate(item.payload);
              palette.close();
            }}
          >
            <div>
              <strong>{item.title}</strong>
              <p>{item.subtitle}</p>
            </div>
            <span>{item.shortcut}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</div>
