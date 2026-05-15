<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import AppIcon from "../components/AppIcon.svelte";
  import SkeletonRow from "../components/SkeletonRow.svelte";
  import { EDITOR_DIRTY_COPY, TASK_PRIORITY_OPTIONS, TASK_STATUS_OPTIONS, TASK_TAB_OPTIONS } from "../config/ui";
  import { tasksStore } from "../stores/tasks";
  import { formatDate } from "../date";
  import {
    formatTaskDayLabel,
    getFocusedTaskItems,
    getTaskPriorityTone,
    getTodayTaskItems
  } from "../tasksPresentation";
  import type { TaskDraftFormDto, TaskItemDto } from "../types";
  const tasks = tasksStore;

  onMount(() => {
    void tasks.load();
  });

  function onTaskFieldChange(field: keyof TaskDraftFormDto, event: Event) {
    const input = event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
    tasks.updateFormField(field, input.value);
  }

  function toggleTaskStatus(task: TaskItemDto) {
    const nextStatus = task.status === "done" ? "open" : "done";
    void tasks.setStatus(task.id, nextStatus);
  }

  let dayLabel = formatTaskDayLabel();
  const dayLabelTimer = setInterval(() => {
    dayLabel = formatTaskDayLabel();
  }, 60_000);
  onDestroy(() => clearInterval(dayLabelTimer));

  let pendingDirtyClose = false;

  function closeEditor(force = false) {
    const result = tasks.closeEditor(force);
    if (result && result.ok === false && result.reason === "dirty") {
      pendingDirtyClose = true;
      return result;
    }

    pendingDirtyClose = false;
    return result;
  }

  function requestClose() {
    closeEditor();
  }

  function confirmDiscardChanges() {
    closeEditor(true);
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
          {#each TASK_TAB_OPTIONS as tab}
            <button class:active={$tasks.tab === tab.value} on:click={() => tasks.setTab(tab.value)}>
              {tab.label}
            </button>
          {/each}
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
        {:else if getFocusedTaskItems($tasks.screen?.items ?? [], $tasks.tab).length === 0}
          <div class="tasks-empty">Задач немає</div>
        {:else}
          {#each getFocusedTaskItems($tasks.screen?.items ?? [], $tasks.tab) as item (item.id)}
            {@const isDone = item.status === "done" || item.status === "cancelled"}
            {@const barColor = getTaskPriorityTone(item.priority)}
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
                      {formatDate(item.dueDate)}
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
          {#each getTodayTaskItems($tasks.screen?.items ?? []) as item (item.id)}
            <button
              class="linked-row"
              on:click={() => tasks.openEditor(item.id)}
            >
              <span class="linked-row-title">{item.title}</span>
              <span class="linked-row-time">{formatDate(item.reminderAt || item.dueDate)}</span>
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
    on:keydown={(e) => { if (e.key === "Enter" || e.key === " ") onBackdropClick(); }}
    role="button"
    tabindex="-1"
    aria-label="Закрити редактор"
  />
  <section class="tasks-editor-sheet" role="dialog" aria-modal="true">
    {#if pendingDirtyClose}
      <div
        class="editor-dirty-banner"
        role="alertdialog"
        aria-live="assertive"
        aria-labelledby="tasks-dirty-banner-title"
        data-testid="tasks-dirty-banner"
      >
        <div>
          <strong id="tasks-dirty-banner-title">{EDITOR_DIRTY_COPY.dirtyTitle}</strong>
          <p>{EDITOR_DIRTY_COPY.dirtyDescription}</p>
        </div>
        <div class="editor-dirty-actions">
          <button
            type="button"
            class="btn-ghost btn-sm"
            on:click={cancelDiscardChanges}
            data-testid="tasks-dirty-banner-cancel"
          >
            {EDITOR_DIRTY_COPY.dirtyStay}
          </button>
          <button
            type="button"
            class="btn-danger btn-sm"
            on:click={confirmDiscardChanges}
            data-testid="tasks-dirty-banner-discard"
          >
            {EDITOR_DIRTY_COPY.dirtyDiscard}
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
          {#each TASK_PRIORITY_OPTIONS as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label>
        Статус
        <select value={$tasks.editor.form.status} on:change={(event) => onTaskFieldChange("status", event)}>
          {#each TASK_STATUS_OPTIONS as option}
            <option value={option.value}>{option.label}</option>
          {/each}
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
