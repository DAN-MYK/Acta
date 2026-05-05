<script lang="ts">
  import { onMount, tick } from "svelte";
  import { appShellStore } from "./lib/stores/app-shell";
  import { navigationStore } from "./lib/stores/navigation";
  import { paletteStore } from "./lib/stores/palette";
  import { settingsStore } from "./lib/stores/settings";
  import { shellStore } from "./lib/stores/shell";
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
  const appShell = appShellStore;
  const shell = shellStore;
  const palette = paletteStore;
  const theme = themeStore;
  const settings = settingsStore;

  const paletteTitleId = "command-palette-title";
  const paletteListId = "command-palette-items";

  let paletteInput: HTMLInputElement | null = null;
  let paletteToggleButton: HTMLButtonElement | null = null;
  let paletteReturnFocusTarget: HTMLElement | null = null;
  let paletteItemButtons: Array<HTMLButtonElement | null> = [];
  let activePaletteIndex = -1;
  let wasPaletteOpen = false;

  const mainNavItems: Array<{
    screen: ScreenId;
    label: string;
    icon: "dashboard" | "documents" | "counterparties" | "payments" | "reports" | "tasks";
    badgeKey?: "documentsBadge" | "tasksBadge";
  }> = [
    { screen: "dashboard", label: "Головна", icon: "dashboard" },
    { screen: "documents", label: "Документи", icon: "documents", badgeKey: "documentsBadge" },
    { screen: "counterparties", label: "Контрагенти", icon: "counterparties" },
    { screen: "payments", label: "Платежі", icon: "payments" },
    { screen: "reports", label: "Звіти", icon: "reports" },
    { screen: "tasks", label: "Завдання", icon: "tasks", badgeKey: "tasksBadge" }
  ];

  const screenTitles: Record<ScreenId, string> = {
    dashboard: "Головна",
    documents: "Документи",
    counterparties: "Контрагенти",
    payments: "Платежі",
    reports: "Звіти",
    tasks: "Завдання",
    settings: "Налаштування"
  };

  const savedFilters = [
    { label: "Прострочені рахунки", count: 1 },
    { label: "Акти без підпису", count: 2 },
    { label: "Неприв'язані платежі", count: 2 },
    { label: "Цей тиждень", count: 7 }
  ];

  onMount(async () => {
    await appShell.bootstrap();
  });

  $: {
    const paletteOpen = $palette.open;
    if (paletteOpen && !wasPaletteOpen) {
      paletteReturnFocusTarget = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      activePaletteIndex = -1;
      void tick().then(() => paletteInput?.focus());
    }

    if (!paletteOpen && wasPaletteOpen) {
      const focusTarget =
        paletteReturnFocusTarget && document.contains(paletteReturnFocusTarget)
          ? paletteReturnFocusTarget
          : paletteToggleButton;
      void tick().then(() => focusTarget?.focus());
      paletteReturnFocusTarget = null;
    }

    wasPaletteOpen = paletteOpen;
  }

  $: {
    const itemsCount = $palette.items.length;
    if (itemsCount === 0) {
      activePaletteIndex = -1;
    } else if (activePaletteIndex >= itemsCount) {
      activePaletteIndex = itemsCount - 1;
    }
  }

  $: document.body.dataset.theme = $theme;
  $: currentScreen = $navigation;
  $: appShellState = $appShell;
  $: shellState = $shell.state;
  $: isShellBusy = $shell.loading || appShellState.loading;
  $: shellProgressLabel = appShellState.progressLabel ?? $shell.progressLabel;
  $: screenTitle = screenTitles[currentScreen] ?? "Acta";
  $: activeCompany = shellState?.companyItems.find(c => c.active);

  function groupLabel(kind: string): string {
    if (kind === 'navigate') return 'Перехід';
    if (kind.startsWith('create_')) return 'Створити';
    if (kind === 'open_document') return 'Документи';
    if (kind === 'open_counterparty') return 'Контрагенти';
    return 'Інше';
  }

  $: groupedPaletteItems = (() => {
    const groups: { label: string; items: typeof $palette.items; startIndex: number }[] = [];
    let currentLabel = '';
    let currentItems: typeof $palette.items = [];
    let startIndex = 0;
    let totalIndex = 0;

    for (const item of $palette.items) {
      const label = groupLabel(item.kind ?? 'navigate');
      if (label !== currentLabel) {
        if (currentItems.length > 0) {
          groups.push({ label: currentLabel, items: currentItems, startIndex });
        }
        currentLabel = label;
        currentItems = [];
        startIndex = totalIndex;
      }
      currentItems.push(item);
      totalIndex++;
    }

    if (currentItems.length > 0) {
      groups.push({ label: currentLabel, items: currentItems, startIndex });
    }

    return groups;
  })();

  function onPaletteInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    activePaletteIndex = -1;
    void palette.search(input.value);
  }

  async function onCompanyChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    if (isShellBusy || shellState?.activeCompanyId === select.value) {
      return;
    }

    await appShell.switchActiveCompany(select.value);
  }

  async function onQuickThemeToggle() {
    const previousMode = $theme;
    const darkMode = previousMode === "light";
    theme.setMode(darkMode ? "dark" : "light");
    settings.updatePreference("darkMode", darkMode);

    const saved = await settings.savePreferences();
    if (saved) {
      appShell.syncThemeFromSettings(saved.screen);
    } else {
      const settingsScreen = await settings.load();
      if (settingsScreen) {
        appShell.syncThemeFromSettings(settingsScreen);
      } else {
        theme.setMode(previousMode);
      }
    }

    await appShell.reloadShellChrome();
  }

  function closePalette() {
    palette.close();
  }

  function focusPaletteItem(index: number) {
    const itemsCount = $palette.items.length;
    if (itemsCount === 0) {
      return;
    }

    const normalizedIndex = ((index % itemsCount) + itemsCount) % itemsCount;
    activePaletteIndex = normalizedIndex;
    void tick().then(() => paletteItemButtons[normalizedIndex]?.focus());
  }

  function activatePaletteItem(index: number) {
    const item = $palette.items[index];
    if (!item) {
      return;
    }

    void palette.activate(item.payload).then(() => {
      closePalette();
    });
  }

  function focusPaletteBoundary(direction: "forward" | "backward") {
    const focusableElements = [
      paletteInput,
      ...paletteItemButtons.filter((button): button is HTMLButtonElement => button !== null)
    ];

    if (focusableElements.length === 0) {
      return;
    }

    if (direction === "forward") {
      focusableElements[0]?.focus();
      activePaletteIndex = -1;
      return;
    }

    const lastElement = focusableElements[focusableElements.length - 1];
    lastElement?.focus();
    activePaletteIndex = focusableElements.length > 1 ? focusableElements.length - 2 : -1;
  }

  function handlePaletteInputKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusPaletteItem(0);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      focusPaletteItem($palette.items.length - 1);
      return;
    }

    if (event.key === "Tab" && event.shiftKey) {
      event.preventDefault();
      focusPaletteBoundary("backward");
    }
  }

  function handlePaletteItemKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusPaletteItem(index + 1);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (index === 0) {
        activePaletteIndex = -1;
        paletteInput?.focus();
        return;
      }

      focusPaletteItem(index - 1);
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      focusPaletteItem(0);
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      focusPaletteItem($palette.items.length - 1);
      return;
    }

    if (event.key === "Tab") {
      event.preventDefault();
      focusPaletteBoundary(event.shiftKey ? "backward" : "forward");
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && $palette.open) {
      event.preventDefault();
      closePalette();
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
    <!-- Brand + Company (об'єднано) -->
    <div class="company-switcher">
      <button class="company-btn" disabled={isShellBusy} on:click={() => {}}>
        <div class="company-logo">
          <svg width="14" height="14" viewBox="0 0 20 20" fill="none">
            <path d="M6 4l4 10M14 4l-4 10M5 5l1 1 2-2" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="company-info">
          <span class="company-name">{shellState?.chrome.companyName ?? "Acta"}</span>
          <span class="company-sub">{activeCompany?.subtitle ?? "Acta · упр. облік"}</span>
        </div>
        <AppIcon name="chevronDown" />
      </button>
      <select
        aria-label="Активна компанія"
        class="sr-only"
        disabled={isShellBusy}
        value={shellState?.activeCompanyId}
        on:change={onCompanyChange}
      >
        {#each shellState?.companyItems ?? [] as company}
          <option value={company.id}>{company.name}</option>
        {/each}
      </select>
    </div>

    <!-- Main nav (36px items, з nav-rail для активних) -->
    <nav class="nav">
      {#each mainNavItems as item}
        {@const badge = item.badgeKey ? shellState?.chrome[item.badgeKey] : undefined}
        <button
          class="nav-item"
          class:active={currentScreen === item.screen}
          data-testid={`nav-${item.screen}`}
          aria-current={currentScreen === item.screen ? "page" : undefined}
          disabled={isShellBusy}
          on:click={() => navigation.go(item.screen)}
        >
          <span class="nav-rail"></span>
          <AppIcon name={item.icon} surface={currentScreen === item.screen} />
          <span>{item.label}</span>
          {#if badge}
            <span class="nav-badge">{badge}</span>
          {/if}
        </button>
      {/each}
    </nav>

    <!-- Saved filters -->
    <div class="saved-filters">
      <div class="saved-filters-header">
        <span>Збережені фільтри</span>
        <button class="saved-filters-add" aria-label="Додати фільтр">+</button>
      </div>
      {#each savedFilters as f}
        <button class="saved-filter-item">
          <span>⭐</span>
          <span class="saved-filter-label">{f.label}</span>
          <span class="saved-filter-count">{f.count}</span>
        </button>
      {/each}
    </div>

    <div class="sidebar-spacer"></div>

    <!-- Settings nav -->
    <button
      class="nav-item"
      class:active={currentScreen === "settings"}
      data-testid="nav-settings"
      aria-current={currentScreen === "settings" ? "page" : undefined}
      disabled={isShellBusy}
      on:click={() => navigation.go("settings")}
      style="margin-bottom: 4px"
    >
      <span class="nav-rail"></span>
      <AppIcon name="settings" surface={currentScreen === "settings"} />
      <span>Налаштування</span>
    </button>

    <!-- User pill -->
    <div class="user-footer">
      <div class="user-avatar" aria-hidden="true">{shellState?.chrome.userInitials ?? "АА"}</div>
      <div class="user-info">
        <span class="user-name">{shellState?.chrome.userName ?? "Користувач"}</span>
        <span class="user-role">{shellState?.chrome.userRole ?? ""}</span>
      </div>
      <button class="user-more" aria-label="Меню користувача">···</button>
    </div>
  </aside>

  <div class="main">
    <header class="topbar">
      <!-- LEFT: title + subtitle -->
      <div class="topbar-left">
        <div class="topbar-title">{screenTitle}</div>
        <div class="topbar-subtitle">
          {#if shellProgressLabel}{shellProgressLabel}{:else}{shellState?.chrome.companyName ?? ""}{/if}
        </div>
      </div>

      <!-- CENTER: search trigger -->
      <button
        bind:this={paletteToggleButton}
        class="topbar-search"
        data-testid="palette-toggle"
        disabled={isShellBusy}
        aria-label="Відкрити палітру команд"
        aria-haspopup="dialog"
        aria-expanded={$palette.open}
        aria-controls={$palette.open ? paletteListId : undefined}
        on:click={() => palette.toggle()}
      >
        <AppIcon name="search" />
        <span class="topbar-search-placeholder">Пошук документа…</span>
        <span class="topbar-search-kbd"><kbd>Ctrl</kbd><kbd>K</kbd></span>
      </button>

      <!-- RIGHT: theme toggle + user avatar -->
      <div class="topbar-right">
        <button
          class="topbar-icon-btn"
          data-testid="theme-toggle"
          disabled={isShellBusy}
          aria-label="Перемкнути тему"
          on:click={onQuickThemeToggle}
        >
          <AppIcon name="theme" surface={true} />
        </button>
        <div class="topbar-user-avatar" aria-hidden="true">
          {shellState?.chrome.userInitials ?? "АА"}
        </div>
      </div>
    </header>

    {#if isShellBusy}
      <div class="shell-progress" aria-live="polite" aria-label={shellProgressLabel ?? "Виконується оновлення shell"}>
        <span></span>
      </div>
    {/if}

    <div class="screen-outlet">
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
    </div>
  </div>

  {#if $palette.open}
    <button
      type="button"
      class="palette-backdrop"
      aria-label="Закрити палітру команд"
      on:click={closePalette}
    ></button>

    <section class="palette" data-testid="palette" role="dialog" aria-modal="true" aria-labelledby={paletteTitleId}>
      <h2 id={paletteTitleId} class="sr-only">Палітра команд</h2>

      <!-- Header row з input -->
      <div class="palette-header">
        <AppIcon name="search" />
        <input
          bind:this={paletteInput}
          type="search"
          placeholder="Пошук команд, екранів і документів"
          aria-label="Пошук команд, екранів і документів"
          aria-controls={paletteListId}
          on:input={onPaletteInput}
          on:keydown={handlePaletteInputKeydown}
        />
        <kbd class="palette-esc">esc</kbd>
      </div>

      <!-- Results grouped by kind -->
      <div class="palette-results" id={paletteListId} data-testid="palette-items">
        {#each groupedPaletteItems as group}
          <div class="palette-group-label">{group.label}</div>
          {#each group.items as item, groupIndex}
            {@const index = group.startIndex + groupIndex}
            <button
              bind:this={paletteItemButtons[index]}
              data-testid={`palette-item-${index}`}
              class="palette-item"
              class:active={activePaletteIndex === index}
              aria-current={activePaletteIndex === index ? "true" : undefined}
              on:click={() => activatePaletteItem(index)}
              on:focus={() => { activePaletteIndex = index; }}
              on:keydown={(event) => handlePaletteItemKeydown(event, index)}
            >
              <span class="palette-item-label">{item.title}</span>
              <span class="palette-item-meta">{item.subtitle}</span>
              <span class="palette-item-shortcut">{item.shortcut}</span>
            </button>
          {/each}
        {/each}
      </div>

      <!-- Footer -->
      <div class="palette-footer">
        <span>↑↓ навігація</span>
        <span>↵ вибрати</span>
        <span class="palette-footer-spacer"></span>
        <span>{$palette.items.length} результатів</span>
      </div>
    </section>
  {/if}
</div>
