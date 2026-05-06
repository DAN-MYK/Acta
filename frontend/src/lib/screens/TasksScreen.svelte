<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import { tasksStore } from "../stores/tasks";
  import type { TaskDraftFormDto, TaskItemDto, TaskStatus } from "../types";

  const tasks = tasksStore;

  onMount(() => {
    void tasks.load();
  });

  function onTaskFieldChange(field: keyof TaskDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    tasks.updateFormField(field, input.value);
  }

  function focusTaskItems(items: TaskItemDto[], tab: "open" | "done" | "all") {
    const scoped =
      tab === "done"
        ? items.filter((item) => item.status === "done" || item.status === "cancelled")
        : tab === "all"
          ? items
          : items.filter((item) => item.status === "open" || item.status === "in_progress");

    return scoped.sort((a, b) => {
      const w = (p: string) => (p === "critical" ? 0 : p === "high" ? 1 : 2);
      if (w(a.priority) !== w(b.priority)) return w(a.priority) - w(b.priority);
      return (a.dueDate || "9999-99-99").localeCompare(b.dueDate || "9999-99-99");
    });
  }

  function todayTaskItems(items: TaskItemDto[]) {
    const today = new Date().toISOString().slice(0, 10);
    return items.filter((item) => item.dueDate === today || item.reminderAt.startsWith(today));
  }

  function toggleTaskStatus(task: TaskItemDto) {
    const nextStatus: TaskStatus = task.status === "done" ? "open" : "done";
    void tasks.setStatus(task.id, nextStatus);
  }

  function priorityBarColor(priority: string): string {
    if (priority === "critical" || priority === "high") return "danger";
    if (priority === "normal") return "warning";
    return "none";
  }

  function computeDayLabel(): string {
    const days = ["нд", "пн", "вт", "ср", "чт", "пт", "сб"];
    const months = ["січ", "лют", "бер", "кві", "тра", "чер", "лип", "сер", "вер", "жов", "лис", "гру"];
    const now = new Date();
    return `${days[now.getDay()]} · ${now.getDate()} ${months[now.getMonth()]}`;
  }

  let dayLabel = computeDayLabel();
  const dayLabelTimer = setInterval(() => {
    dayLabel = computeDayLabel();
  }, 60_000);
  onDestroy(() => clearInterval(dayLabelTimer));

  let pendingDirtyClose = false;

  function requestClose() {
    const result = tasks.closeEditor();
    if (result && result.ok === false && result.reason === "dirty") {
      pendingDirtyClose = true;
      return;
    }
    pendingDirtyClose = false;
  }

  function confirmDiscardChanges() {
    pendingDirtyClose = false;
    tasks.closeEditor(true);
  }

  function cancelDiscardChanges() {
    pendingDirtyClose = false;
  }

  function onBackdropClick() {
    requestClose();
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if ($tasks.editor && event.key === "Escape") {
      requestClose();
    }
  }

  $: if (!$tasks.editor && pendingDirtyClose) {
    pendingDirtyClose = false;
  }
</script>

<section
  class="panel tasks-panel"
  data-testid="tasks-screen"
  inert={$tasks.editor ? true : undefined}
>
  <!-- KPI strip -->
  <div class="task-kpis">
    {#if $tasks.initialLoading}
      <div class="sk tasks-kpi-skeleton" />
    {:else}
      <div class="kpi-cell">
        <span class="kpi-label">Відкритих</span>
        <span class="kpi-value">{$tasks.screen?.openCount ?? 0}</span>
      </div>
      <div class="kpi-divider" />
      <div class="kpi-cell">
        <span class="kpi-label">Високий пріоритет</span>
        <span class="kpi-value" class:kpi-danger={($tasks.screen?.highCount ?? 0) > 0}>
          {$tasks.screen?.highCount ?? 0}
        </span>
      </div>
      <div class="kpi-divider" />
      <div class="kpi-cell">
        <span class="kpi-label">Виконано</span>
        <span class="kpi-value">{$tasks.screen?.doneCount ?? 0}</span>
      </div>
      <div class="kpi-divider" />
      <div class="kpi-cell">
        <span class="kpi-label">Сьогодні</span>
        <span class="kpi-value">{$tasks.screen?.todayCount ?? 0}</span>
      </div>
    {/if}
  </div>

  {#if $tasks.message}
    <div class="tasks-message">{$tasks.message}</div>
  {/if}
  {#if $tasks.error}
    <div class="tasks-error">{$tasks.error}</div>
  {/if}

  <!-- Two-column layout -->
  <div class="tasks-layout">
    <!-- Main list card -->
    <div class="tasks-main tasks-card">
      <div class="tasks-card-header">
        <h3>Завдання</h3>
        <div class="task-tabs" data-testid="tasks-focus-primary">
          <button class:active={$tasks.tab === "open"} on:click={() => tasks.setTab("open")}>
            У фокусі
          </button>
          <button class:active={$tasks.tab === "done"} on:click={() => tasks.setTab("done")}>
            Завершені
          </button>
          <button class:active={$tasks.tab === "all"} on:click={() => tasks.setTab("all")}>
            Усі
          </button>
        </div>
        <button class="btn-primary btn-sm tasks-new-btn" on:click={() => tasks.openEditor()}>
          <AppIcon name="add" size={13} />
          Нове завдання
        </button>
      </div>

      <div class="tasks-list" data-testid="tasks-list">
        {#if $tasks.initialLoading}
          <div class="tasks-skeleton-wrapper">
            <SkeletonRow count={5} variant="compact" />
          </div>
        {:else if focusTaskItems($tasks.screen?.items ?? [], $tasks.tab).length === 0}
          <div class="tasks-empty">Задач немає</div>
        {:else}
          {#each focusTaskItems($tasks.screen?.items ?? [], $tasks.tab) as item (item.id)}
            {@const isDone = item.status === "done" || item.status === "cancelled"}
            {@const barColor = priorityBarColor(item.priority)}
            <div class="task-row" class:task-row-done={isDone}>
              {#if barColor !== "none" && !isDone}
                <div class="task-priority-bar task-priority-{barColor}" />
              {:else}
                <div class="task-priority-bar task-priority-none" />
              {/if}

              <button class="task-row-main" on:click={() => tasks.openEditor(item.id)}>
                <div class="task-row-content">
                  <strong class="task-row-title">{item.title}</strong>
                  {#if item.linkLabel}
                    <span class="task-row-link">Пов'язано з {item.linkLabel}</span>
                  {/if}
                </div>
                <div class="task-row-meta">
                  {#if item.dueDate}
                    <span class="task-meta-date">
                      <AppIcon name="calendar" size={10} />
                      {item.dueDate}
                    </span>
                  {/if}
                  <span class="task-pill task-pill-{item.priority}">{item.priorityLabel}</span>
                  <span class="task-status-label">{item.statusLabel}</span>
                </div>
              </button>

              <button class="btn-secondary task-row-status" on:click={() => toggleTaskStatus(item)}>
                {isDone ? "Повернути" : "Готово"}
              </button>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    <!-- Today panel -->
    <aside class="tasks-side-panel task-today-panel tasks-card" data-testid="tasks-today-panel">
      <div class="today-header">
        <h3>На сьогодні</h3>
        <span class="today-day">{dayLabel}</span>
      </div>

      <div class="linked-list">
        {#if $tasks.initialLoading}
          <div data-testid="tasks-today-skeleton">
            <SkeletonRow count={3} variant="compact" />
          </div>
        {:else}
          {#each todayTaskItems($tasks.screen?.items ?? []) as item (item.id)}
            <button
              class="linked-row"
              on:click={() => tasks.openEditor(item.id)}
            >
              <span class="linked-row-title">{item.title}</span>
              <span class="linked-row-time">{item.reminderAt || item.dueDate}</span>
            </button>
          {:else}
            <div class="empty-state-card compact">
              <strong>Сьогодні немає нагадувань</strong>
              <p>Можна спокійно планувати нові задачі або закрити хвости з попередніх днів.</p>
            </div>
          {/each}
        {/if}
      </div>
    </aside>
  </div>
</section>

<svelte:window on:keydown={onWindowKeydown} />

<!-- Editor drawer -->
{#if $tasks.editor}
  <div
    class="editor-backdrop"
    on:click={onBackdropClick}
    role="button"
    tabindex="-1"
    aria-label="Закрити редактор"
  />
  <section class="editor-sheet" role="dialog" aria-modal="true">
    {#if pendingDirtyClose}
      <div
        class="editor-dirty-banner"
        role="alertdialog"
        aria-live="assertive"
        aria-labelledby="tasks-dirty-banner-title"
        data-testid="tasks-dirty-banner"
      >
        <div>
          <strong id="tasks-dirty-banner-title">У вас є незбережені зміни</strong>
          <p>Скасувати їх і закрити форму?</p>
        </div>
        <div class="editor-dirty-actions">
          <button
            type="button"
            class="btn-ghost btn-sm"
            on:click={cancelDiscardChanges}
            data-testid="tasks-dirty-banner-cancel"
          >
            Залишитися
          </button>
          <button
            type="button"
            class="btn-danger btn-sm"
            on:click={confirmDiscardChanges}
            data-testid="tasks-dirty-banner-discard"
          >
            Так, закрити
          </button>
        </div>
      </div>
    {/if}
    <div class="editor-header">
      <div>
        <h3>{$tasks.editor.title}</h3>
        {#if $tasks.editor.form.linkLabel}
          <p class="editor-link">Пов'язано з {$tasks.editor.form.linkLabel}</p>
        {:else}
          <p class="editor-link editor-link-none">Без прив'язки</p>
        {/if}
      </div>
      <div class="editor-actions">
        <button
          class="btn-primary btn-sm"
          disabled={$tasks.loading || !$tasks.editor.form.title.trim()}
          on:click={() => tasks.save()}
        >
          <AppIcon name="save" size={13} />
          Зберегти
        </button>
        {#if $tasks.editor.form.id}
          <button
            class="btn-danger btn-sm"
            disabled={$tasks.loading}
            on:click={() => tasks.deleteCurrent()}
          >
            <AppIcon name="delete" size={13} />
            Видалити
          </button>
        {/if}
        <button class="btn-ghost btn-sm editor-close" on:click={requestClose}>
          <AppIcon name="close" size={15} />
        </button>
      </div>
    </div>

    <div class="editor-grid">
      <label class="editor-grid-span">
        Назва <span class="required">*</span>
        <input
          type="text"
          value={$tasks.editor.form.title}
          placeholder="Назва задачі…"
          on:input={(event) => onTaskFieldChange("title", event)}
        />
      </label>
      <label class="editor-grid-span">
        Опис
        <textarea
          rows="3"
          value={$tasks.editor.form.description}
          placeholder="Деталі задачі…"
          on:input={(event) => onTaskFieldChange("description", event)}
        ></textarea>
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
        <input
          type="date"
          value={$tasks.editor.form.dueDate}
          on:change={(event) => onTaskFieldChange("dueDate", event)}
        />
      </label>
      <label>
        Нагадування
        <input
          type="datetime-local"
          value={$tasks.editor.form.reminderAt}
          on:change={(event) => onTaskFieldChange("reminderAt", event)}
        />
      </label>
    </div>
  </section>
{/if}

<style>
  .tasks-panel {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 20px 22px 36px;
  }

  /* KPI strip */
  .task-kpis {
    display: flex;
    align-items: stretch;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }

  .kpi-cell {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px 20px;
  }

  .kpi-divider {
    width: 1px;
    background: var(--border);
    margin: 12px 0;
    flex-shrink: 0;
  }

  .kpi-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 1.1px;
  }

  .kpi-value {
    font-size: 24px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    line-height: 1.05;
    color: var(--text);
    font-family: var(--font-sans);
  }

  .kpi-value.kpi-danger {
    color: var(--danger);
  }

  /* Layout grid */
  .tasks-layout {
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 14px;
    align-items: start;
  }

  /* Cards */
  .tasks-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
  }

  /* Card header */
  .tasks-card-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .tasks-card-header h3 {
    margin: 0;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
  }

  /* Tabs */
  .task-tabs {
    display: flex;
    padding: 2px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    margin-left: 4px;
  }

  .task-tabs button {
    padding: 3px 9px;
    background: transparent;
    color: var(--text-muted);
    border: none;
    cursor: pointer;
    border-radius: 4px;
    font-size: 11.5px;
    font-weight: 400;
    white-space: nowrap;
    transition: background 100ms, color 100ms;
  }

  .task-tabs button.active {
    background: var(--bg-elevated);
    color: var(--text);
    font-weight: 500;
    box-shadow: 0 0 0 1px var(--border);
  }

  /* Task list */
  .tasks-list {
    min-height: 80px;
  }

  /* Task row */
  .task-row {
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--border);
    transition: background 100ms;
    min-height: 52px;
  }

  .task-row:hover {
    background: var(--bg-hover);
  }

  .task-row:last-child {
    border-bottom: none;
  }

  .task-row-done {
    opacity: 0.55;
  }

  /* Priority bar */
  .task-priority-bar {
    width: 3px;
    flex-shrink: 0;
  }

  .task-priority-danger {
    background: var(--danger);
  }

  .task-priority-warning {
    background: var(--warning);
  }

  .task-priority-none {
    background: transparent;
  }

  /* Row main button */
  .task-row-main {
    flex: 1;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 11px 14px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    color: inherit;
    min-width: 0;
    flex-direction: column;
  }

  .task-row-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .task-row-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    display: block;
  }

  .task-row-done .task-row-title {
    text-decoration: line-through;
    color: var(--text-faint);
  }

  .task-row-link {
    font-size: 11px;
    color: var(--accent-text);
    font-family: var(--font-mono);
  }

  .task-row-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 3px;
  }

  .task-meta-date {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }

  .task-pill {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--bg-subtle);
    color: var(--text-muted);
  }

  .task-pill-high,
  .task-pill-critical {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .task-status-label {
    font-size: 11px;
    color: var(--text-faint);
  }

  /* Status toggle button */
  .task-row-status {
    align-self: center;
    margin: 0 12px 0 8px;
    flex-shrink: 0;
    font-size: 11.5px;
    padding: 4px 10px;
    height: 26px;
    white-space: nowrap;
  }

  /* Empty / feedback states */
  .tasks-empty {
    padding: 48px 20px;
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
  }

  .tasks-message {
    padding: 10px 16px;
    color: var(--success);
    font-size: 12px;
    background: var(--success-soft);
    border-radius: 8px;
  }

  .tasks-error {
    padding: 10px 16px;
    color: var(--danger);
    font-size: 12px;
    background: var(--danger-soft);
    border-radius: 8px;
  }

  /* Today panel */
  .task-today-panel {
    display: flex;
    flex-direction: column;
  }

  .today-header {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .today-header h3 {
    margin: 0 0 2px;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text);
  }

  .today-day {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .linked-list {
    display: flex;
    flex-direction: column;
  }

  .linked-row {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 16px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
    transition: background 100ms;
  }

  .linked-row:hover {
    background: var(--bg-hover);
  }

  .linked-row:last-child {
    border-bottom: none;
  }

  .linked-row-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text);
  }

  .linked-row-time {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .empty-state-card.compact {
    padding: 24px 16px;
    text-align: center;
  }

  .empty-state-card.compact strong {
    display: block;
    font-size: 13px;
    color: var(--text-muted);
    font-weight: 500;
    margin-bottom: 6px;
  }

  .empty-state-card.compact p {
    font-size: 12px;
    color: var(--text-faint);
    line-height: 1.5;
    margin: 0;
  }

  .tasks-new-btn {
    margin-left: auto;
  }

  .tasks-kpi-skeleton {
    height: 64px;
    border-radius: 10px;
    grid-column: 1 / -1;
  }

  .tasks-skeleton-wrapper {
    padding: 12px 16px;
  }

  /* Editor drawer */
  .editor-backdrop {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: var(--acta-color-bg-overlay);
    backdrop-filter: blur(2px);
    border: none;
    padding: 0;
    cursor: default;
  }

  .editor-sheet {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 201;
    width: 480px;
    max-width: 100vw;
    background: var(--bg-elevated);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    box-shadow: -6px 0 28px rgba(0, 0, 0, 0.1);
  }

  .editor-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 18px 20px 14px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .editor-header h3 {
    margin: 0 0 4px;
    font-size: 16px;
    font-weight: 500;
    color: var(--text);
  }

  .editor-link {
    margin: 0;
    font-size: 11.5px;
    color: var(--accent-text);
    font-family: var(--font-mono);
  }

  .editor-link-none {
    color: var(--text-faint);
    font-family: var(--font-sans);
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .editor-close {
    width: 28px;
    height: 28px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .editor-grid {
    flex: 1;
    overflow-y: auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    padding: 20px;
    align-content: start;
  }

  .editor-grid-span {
    grid-column: 1 / -1;
  }

  .editor-grid label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-muted);
    font-weight: 500;
  }

  .editor-grid input,
  .editor-grid select,
  .editor-grid textarea {
    padding: 7px 10px;
    font-size: 13px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 5px;
    outline: none;
    font-family: var(--font-sans);
    transition: border-color 120ms;
    width: 100%;
    box-sizing: border-box;
  }

  .editor-grid input:focus,
  .editor-grid select:focus,
  .editor-grid textarea:focus {
    border-color: var(--accent);
  }

  .editor-grid textarea {
    resize: vertical;
    min-height: 68px;
  }

  .required {
    color: var(--danger);
    font-weight: 400;
  }
</style>
