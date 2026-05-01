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

  function focusTaskItems(items: TaskItemDto[], tab: "open" | "done" | "all") {
    const scopedItems = taskItemsForTab(items, tab);

    return scopedItems.sort((left, right) => {
      const leftWeight = left.priority === "critical" ? 0 : left.priority === "high" ? 1 : 2;
      const rightWeight = right.priority === "critical" ? 0 : right.priority === "high" ? 1 : 2;

      if (leftWeight !== rightWeight) {
        return leftWeight - rightWeight;
      }

      return (left.dueDate || "9999-99-99").localeCompare(right.dueDate || "9999-99-99");
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

  function linkLabel(item: TaskItemDto) {
    return item.linkLabel ? `Пов'язано з ${item.linkLabel}` : "Без прив'язки";
  }
</script>

<section class="panel" data-testid="tasks-screen">
  <div class="panel-header">
    <div>
      <h2>Завдання</h2>
      <p>{$tasks.screen?.items.length ?? 0} записів у поточній вибірці</p>
    </div>
    <div class="panel-actions">
      <input placeholder="Пошук завдань" on:input={onTaskSearch} />
      <button class="btn-primary" on:click={() => tasks.openEditor()}>Нове завдання</button>
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
      <div class="task-focus-card" data-testid="tasks-focus-primary">
        <strong>У фокусі</strong>
        <p>Потребують уваги зараз: прострочені, високопріоритетні або прив'язані до грошових рішень завдання.</p>
      </div>

      <div class="task-tabs">
        <button class:active={$tasks.tab === "open"} on:click={() => tasks.setTab("open")}>У фокусі</button>
        <button class:active={$tasks.tab === "done"} on:click={() => tasks.setTab("done")}>Завершені</button>
        <button class:active={$tasks.tab === "all"} on:click={() => tasks.setTab("all")}>Усі</button>
      </div>

      <div class="tasks-list" data-testid="tasks-list">
        {#each focusTaskItems($tasks.screen?.items ?? [], $tasks.tab) as item}
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
              <div class="task-row-context">
                <span>{linkLabel(item)}</span>
                {#if item.reminderAt}
                  <span>Нагадування {item.reminderAt}</span>
                {/if}
              </div>
            </button>
            <button class="btn-secondary task-row-status" on:click={() => toggleTaskStatus(item)}>
              {item.status === "done" ? "Повернути" : "Готово"}
            </button>
          </div>
        {/each}
      </div>
    </div>

    <aside class="tasks-side-panel task-today-panel" data-testid="tasks-today-panel">
      <strong>На сьогодні</strong>
      <p>Швидкий список задач, які спливають сьогодні або мають нагадування на поточну дату.</p>
      <div class="linked-list">
        {#each todayTaskItems($tasks.screen?.items ?? []) as item}
          <button class="linked-row" on:click={() => tasks.openEditor(item.id)}>
            <span>{item.title}</span>
            <span>{item.reminderAt || item.dueDate}</span>
          </button>
        {:else}
          <div class="empty-state-card compact">
            <strong>Сьогодні немає нагадувань</strong>
            <p>Можна спокійно планувати нові задачі або закрити хвости з попередніх днів.</p>
          </div>
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
        <p>{$tasks.editor.form.linkLabel ? `Пов'язано з ${$tasks.editor.form.linkLabel}` : "Без прив'язки"}</p>
      </div>
      <div class="editor-actions">
        <button class="btn-primary" on:click={() => tasks.save()}>Зберегти</button>
        {#if $tasks.editor.form.id}
          <button class="btn-danger" on:click={() => tasks.deleteCurrent()}>Видалити</button>
        {/if}
        <button class="btn-ghost" on:click={() => tasks.closeEditor()}>Закрити</button>
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
