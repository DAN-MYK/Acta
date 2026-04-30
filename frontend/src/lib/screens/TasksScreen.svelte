<script lang="ts">
  import { tasksStore } from "../stores/tasks";
  import type { TaskDraftFormDto, TaskItemDto, TaskStatus } from "../types";

  const tasks = tasksStore;

  function onTaskSearch(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void tasks.load(input.value);
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
</script>

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

{#if $tasks.editor}
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
