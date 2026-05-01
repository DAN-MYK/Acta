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
  import TasksScreen from "./lib/screens/TasksScreen.svelte";
  import AppIcon from "./lib/components/AppIcon.svelte";
  import type { ScreenId } from "./lib/types";

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
  let isCompanyReloading = false;

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
  $: isShellBusy = $shell.loading || isCompanyReloading;
  $: themeLabel = $theme === "dark" ? "темна" : "світла";

  function onPaletteInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void palette.search(input.value);
  }

  async function onCompanyChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    if (isShellBusy) {
      return;
    }

    isCompanyReloading = true;

    try {
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
    } finally {
      isCompanyReloading = false;
    }
  }

  async function onQuickThemeToggle() {
    const previousMode = $theme;
    const darkMode = previousMode === "light";
    theme.setMode(darkMode ? "dark" : "light");
    settings.updatePreference("darkMode", darkMode);

    const saved = await settings.savePreferences();
    if (saved) {
      theme.setMode(saved.screen.preferences.darkMode ? "dark" : "light");
    } else {
      const settingsScreen = await settings.load();
      theme.setMode(settingsScreen?.preferences.darkMode ? "dark" : previousMode);
    }

    const shellState = await shell.load();
    if (shellState) {
      theme.setMode(shellState.isDark ? "dark" : "light");
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && $palette.open) {
      event.preventDefault();
      palette.close();
      return;
    }

    if (event.ctrlKey && event.key.toLowerCase() === "k") {
      event.preventDefault();
      if (isShellBusy) {
        return;
      }
      palette.toggle();
    }

    if (event.ctrlKey && event.key >= "1" && event.key <= "7") {
      event.preventDefault();
      if (isShellBusy) {
        return;
      }
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
        <p>Управлінський облік</p>
      </div>
    </div>

    <nav class="nav">
      {#each sidebarScreens as item}
        <button
          data-testid={`nav-${item.screen}`}
          class:active={currentScreen === item.screen}
          disabled={isShellBusy}
          on:click={() => navigation.go(item.screen)}
        >
          <AppIcon name={item.icon} surface={currentScreen === item.screen} />
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>

    <div class="theme-switcher">
      <button disabled={isShellBusy} on:click={onQuickThemeToggle}>
        <AppIcon name="theme" surface={true} />
        <span>Тема: {themeLabel}</span>
      </button>
    </div>
  </aside>

  <main class="content">
    <header class:busy={isShellBusy} class="topbar">
      <div>
        <h1>{shellState?.chrome.companyName ?? "Acta"}</h1>
        <p>
          {#if isCompanyReloading}
            Оновлюємо дані активної компанії...
          {:else if $shell.loading}
            Завантажуємо shell...
          {:else}
            {shellState?.chrome.userRole ?? "Завантаження shell..."}
          {/if}
        </p>
      </div>

      <div class="topbar-actions">
        <select disabled={isShellBusy} value={shellState?.activeCompanyId} on:change={onCompanyChange}>
          {#each shellState?.companyItems ?? [] as company}
            <option value={company.id}>{company.name}</option>
          {/each}
        </select>

        <button data-testid="palette-toggle" disabled={isShellBusy} on:click={() => palette.toggle()}>
          <AppIcon name="palette" surface={true} />
          <span>Ctrl+K</span>
        </button>
      </div>
    </header>

    {#if isShellBusy}
      <div class="shell-progress" aria-live="polite" aria-label="Виконується оновлення shell">
        <span></span>
      </div>
    {/if}

    {#if currentScreen === "dashboard"}
      <DashboardScreen />
    {:else if currentScreen === "documents"}
      <DocumentsScreen />
    {:else if currentScreen === "counterparties"}
      <CounterpartiesScreen />
    {:else if currentScreen === "reports"}
      <ReportsScreen />
    {:else if currentScreen === "tasks"}
      <TasksScreen />
    {:else if currentScreen === "settings"}
      <SettingsScreen />
    {:else if currentScreen === "payments"}
      <PaymentsScreen />
    {:else}
      <section class="panel empty-screen">
        <h2>Невідомий екран</h2>
        <p>Сторінку не знайдено.</p>
      </section>
    {/if}
  </main>

  {#if $palette.open}
    <button type="button" class="palette-backdrop" aria-label="Закрити палітру команд" on:click={() => palette.close()}></button>
    <section class="palette" data-testid="palette">
      <input bind:this={paletteInput} placeholder="Пошук команд, екранів і документів" on:input={onPaletteInput} />

      <div class="palette-items" data-testid="palette-items">
        {#each $palette.items as item, index}
          <button
            data-testid={`palette-item-${index}`}
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
