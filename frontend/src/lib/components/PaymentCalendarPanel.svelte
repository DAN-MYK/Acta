<script lang="ts">
  import {
    CALENDAR_EVENT_KIND_LABELS,
    CALENDAR_FILTER_OPTIONS,
    formatCalendarDayAriaLabel,
    getCalendarEventDirectionLabel
  } from "../config/ui";
  import { paymentsStore } from "../stores/payments";
  import type {
    PaymentCalendarDayDto,
    PaymentCalendarEventDto,
    PaymentCalendarEventKind
  } from "../types";

  const payments = paymentsStore;
  const calendarFilterOptions = CALENDAR_FILTER_OPTIONS;

  function filteredEvents(day: PaymentCalendarDayDto) {
    if ($payments.calendarFilter === "all") {
      return day.events;
    }

    return day.events.filter((event) => event.kind === $payments.calendarFilter);
  }

  function countEventsByKind(kind: PaymentCalendarEventKind) {
    if (!$payments.calendar) {
      return 0;
    }

    return $payments.calendar.days.reduce(
      (sum, day) => sum + day.events.filter((event) => event.kind === kind).length,
      0
    );
  }

  function eventTone(event: PaymentCalendarEventDto) {
    if (event.done) {
      return "done";
    }
    if (event.overdue) {
      return "overdue";
    }
    return event.kind;
  }

  function eventKindLabel(event: PaymentCalendarEventDto) {
    return CALENDAR_EVENT_KIND_LABELS[event.kind];
  }

  function eventDirectionLabel(event: PaymentCalendarEventDto) {
    if (event.kind !== "schedule") {
      return "";
    }

    return getCalendarEventDirectionLabel(event.direction);
  }

  function dayAriaLabel(day: PaymentCalendarDayDto) {
    return formatCalendarDayAriaLabel({
      date: day.date,
      eventCount: filteredEvents(day).length,
      today: day.today,
      selected: day.selected
    });
  }

  function onGridKeydown(event: KeyboardEvent) {
    if (!$payments.calendar) {
      return;
    }

    if (event.key === "ArrowLeft") {
      event.preventDefault();
      void payments.moveCalendarSelection(-1);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      void payments.moveCalendarSelection(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      void payments.moveCalendarSelection(-7);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      void payments.moveCalendarSelection(7);
    }
  }

  $: weekdayLabels = $payments.calendar?.days.slice(0, 7).map((day) => day.weekdayShort) ?? [
    "Пн",
    "Вт",
    "Ср",
    "Чт",
    "Пт",
    "Сб",
    "Нд"
  ];
  $: selectedDay = $payments.calendar?.days.find((day) => day.selected) ?? null;
  $: selectedEvents = selectedDay ? filteredEvents(selectedDay) : [];
  $: selectedEvent =
    selectedEvents.find((event) => event.id === $payments.selectedCalendarEventId) ??
    selectedEvents[0] ??
    null;
  $: visibleEventCount = $payments.calendar
    ? $payments.calendar.days.reduce((sum, day) => sum + filteredEvents(day).length, 0)
    : 0;
  $: scheduleCount = countEventsByKind("schedule");
  $: taskCount = countEventsByKind("task");
</script>

<section class="calendar-shell" data-testid="payments-calendar">
  <div class="calendar-shell-header">
    <div>
      <h3>Платіжний календар</h3>
      <p>Місячна сітка показує планові платежі та дедлайни задач в одному часовому контексті.</p>
    </div>
    <div class="calendar-toolbar">
      <div class="calendar-nav">
        <button class="btn-ghost" on:click={() => payments.shiftCalendarMonth(-1)} disabled={$payments.calendarLoading}>
          Попередній
        </button>
        <strong>{$payments.calendar?.monthLabel ?? "Завантажуємо місяць"}</strong>
        <button class="btn-ghost" on:click={() => payments.shiftCalendarMonth(1)} disabled={$payments.calendarLoading}>
          Наступний
        </button>
      </div>
      <div class="calendar-filters" role="radiogroup" aria-label="Фільтр подій календаря">
        {#each calendarFilterOptions as option}
          <button
            class:active={$payments.calendarFilter === option.kind}
            on:click={() => payments.setCalendarFilter(option.kind)}
            role="radio"
            aria-checked={$payments.calendarFilter === option.kind}
          >
            {option.label}
          </button>
        {/each}
      </div>
    </div>
  </div>

  <div class="calendar-summary">
    <div class="calendar-summary-card">
      <strong>{scheduleCount}</strong>
      <span>Планових платежів у місяці</span>
    </div>
    <div class="calendar-summary-card">
      <strong>{taskCount}</strong>
      <span>Дедлайнів задач у місяці</span>
    </div>
    <div class="calendar-summary-card">
      <strong>{visibleEventCount}</strong>
      <span>Подій у поточному фільтрі</span>
    </div>
  </div>

  {#if $payments.calendarError && !$payments.calendar}
    <div class="empty-state-card" data-testid="payments-calendar-error" role="alert">
      <strong>Календар не завантажився</strong>
      <p>{$payments.calendarError}</p>
      <button class="btn-secondary" on:click={() => payments.loadCalendar()}>Спробувати ще раз</button>
    </div>
  {:else if $payments.calendarInitialLoading && !$payments.calendar}
    <div class="calendar-loading" data-testid="payments-calendar-loading">
      <div class="calendar-loading-grid"></div>
      <div class="calendar-loading-side"></div>
    </div>
  {:else if !$payments.calendar}
    <div class="empty-state-card" data-testid="payments-calendar-empty">
      <strong>Календар поки порожній</strong>
      <p>Коли з’являться події графіка платежів або задачі з дедлайнами, вони відобразяться тут.</p>
    </div>
  {:else}
    <div class="calendar-layout">
      <div class="calendar-grid-panel">
        <div class="calendar-weekdays">
          {#each weekdayLabels as label}
            <span>{label}</span>
          {/each}
        </div>

        <div
          class="calendar-grid"
          role="grid"
          tabindex="0"
          aria-label="Місячна сітка платіжного календаря"
          on:keydown={onGridKeydown}
        >
          {#each $payments.calendar.days as day}
            <button
              class="calendar-day"
              class:is-muted={!day.inCurrentMonth}
              class:is-selected={day.selected}
              class:has-overdue={day.hasOverdue}
              on:click={() => payments.selectCalendarDate(day.date)}
              aria-pressed={day.selected}
              aria-label={dayAriaLabel(day)}
            >
              <div class="calendar-day-top">
                <strong>{day.dayNumber}</strong>
                {#if filteredEvents(day).length > 0}
                  <span>{filteredEvents(day).length}</span>
                {/if}
              </div>

              <div class="calendar-day-totals">
                {#if day.incomeTotalStr}
                  <span class="calendar-money is-positive">+{day.incomeTotalStr}</span>
                {/if}
                {#if day.expenseTotalStr}
                  <span class="calendar-money is-negative">-{day.expenseTotalStr}</span>
                {/if}
              </div>

              <div class="calendar-day-events">
                {#each filteredEvents(day).slice(0, 2) as event}
                  <span class={`calendar-pill is-${eventTone(event)}`}>
                    {event.title}
                  </span>
                {/each}
                {#if filteredEvents(day).length > 2}
                  <span class="calendar-pill is-more">+{filteredEvents(day).length - 2} ще</span>
                {/if}
              </div>
            </button>
          {/each}
        </div>

        {#if visibleEventCount === 0}
          <div class="empty-state-card compact" data-testid="payments-calendar-filter-empty">
            <strong>У цьому місяці немає подій для поточного фільтра</strong>
            <p>Перемкніть фільтр або перейдіть на інший місяць, щоб подивитися інші записи.</p>
          </div>
        {/if}
      </div>

      <aside class="calendar-side-panel" data-testid="payments-calendar-details">
        <div class="calendar-side-header">
          <div>
            <strong>{selectedDay?.date ?? "День не вибрано"}</strong>
            <p>
              {#if selectedEvents.length > 0}
                {selectedEvents.length} подій у вибраному дні
              {:else}
                На цей день немає подій у поточному фільтрі
              {/if}
            </p>
          </div>
          {#if $payments.calendarLoading}
            <span class="task-pill">Оновлюємо</span>
          {/if}
        </div>

        {#if selectedEvents.length > 0}
          <div class="calendar-event-list">
            {#each selectedEvents as event}
              <button
                class="calendar-event-row"
                class:is-selected={$payments.selectedCalendarEventId === event.id}
                on:click={() => payments.selectCalendarEvent(event.id)}
              >
                <div>
                  <strong>{event.title}</strong>
                  <p>{event.subtitle}</p>
                </div>
                <div class="calendar-event-meta">
                  <span class={`task-pill is-${eventTone(event)}`}>{eventKindLabel(event)}</span>
                  <span>{event.statusLabel}</span>
                </div>
              </button>
            {/each}
          </div>

          {#if selectedEvent}
            <div class="calendar-event-detail">
              <div class="calendar-event-detail-top">
                <div>
                  <strong>{selectedEvent.title}</strong>
                  <p>{selectedEvent.subtitle}</p>
                </div>
                <span class={`task-pill is-${eventTone(selectedEvent)}`}>{selectedEvent.statusLabel}</span>
              </div>

              <dl class="calendar-event-facts">
                <div>
                  <dt>Тип</dt>
                  <dd>{eventKindLabel(selectedEvent)}</dd>
                </div>
                {#if selectedEvent.amountStr}
                  <div>
                    <dt>Сума</dt>
                    <dd>{selectedEvent.amountStr}</dd>
                  </div>
                {/if}
                {#if selectedEvent.kind === "schedule"}
                  <div>
                    <dt>Напрям</dt>
                    <dd>{eventDirectionLabel(selectedEvent)}</dd>
                  </div>
                {/if}
                {#if selectedEvent.recurrenceLabel}
                  <div>
                    <dt>Повтор</dt>
                    <dd>{selectedEvent.recurrenceLabel}</dd>
                  </div>
                {/if}
                {#if selectedEvent.counterpartyName}
                  <div>
                    <dt>Контрагент</dt>
                    <dd>{selectedEvent.counterpartyName}</dd>
                  </div>
                {/if}
              </dl>

              {#if selectedEvent.note}
                <div class="calendar-note">
                  <strong>Примітка</strong>
                  <p>{selectedEvent.note}</p>
                </div>
              {/if}

              <div class="calendar-event-actions">
                {#if selectedEvent.kind === "task"}
                  <button class="btn-primary" on:click={() => payments.openCalendarTask(selectedEvent.id)}>
                    Відкрити задачу
                  </button>
                {/if}

                {#if selectedEvent.kind === "schedule"}
                  <button class="btn-secondary" on:click={() => payments.createPaymentFromSchedule(selectedEvent)}>
                    Створити платіж
                  </button>
                {/if}

                {#if selectedEvent.kind === "schedule" && selectedEvent.actionable}
                  <button
                    class="btn-primary"
                    on:click={() => payments.completeSchedule(selectedEvent.id)}
                    disabled={$payments.loading && $payments.activePaymentId === selectedEvent.id}
                  >
                    Позначити виконаним
                  </button>
                {/if}

                {#if selectedEvent.counterpartyId}
                  <button
                    class="btn-ghost"
                    on:click={() => payments.openCalendarCounterparty(selectedEvent.counterpartyId)}
                  >
                    До контрагента
                  </button>
                {/if}
              </div>
            </div>
          {/if}
        {:else}
          <div class="empty-state-card compact">
            <strong>На цей день подій не знайдено</strong>
            <p>Оберіть інший день, змініть фільтр або перейдіть на інший місяць.</p>
          </div>
        {/if}
      </aside>
    </div>
  {/if}
</section>

<style>
  .calendar-shell {
    margin-top: 18px;
    padding: 18px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
    box-shadow: var(--acta-shadow-card);
    display: grid;
    gap: 18px;
  }

  .calendar-shell-header,
  .calendar-toolbar,
  .calendar-nav,
  .calendar-summary,
  .calendar-layout,
  .calendar-side-header,
  .calendar-event-row,
  .calendar-event-detail-top,
  .calendar-event-actions {
    display: flex;
    gap: 12px;
  }

  .calendar-shell-header,
  .calendar-side-header,
  .calendar-event-row,
  .calendar-event-detail-top {
    justify-content: space-between;
    align-items: flex-start;
  }

  .calendar-shell-header h3,
  .calendar-side-header strong,
  .calendar-event-detail strong {
    margin: 0;
  }

  .calendar-shell-header p,
  .calendar-side-header p,
  .calendar-event-row p,
  .calendar-event-detail p,
  .calendar-note p {
    margin: 4px 0 0;
    color: var(--acta-color-text-muted);
  }

  .calendar-toolbar,
  .calendar-nav {
    align-items: center;
    flex-wrap: wrap;
  }

  .calendar-toolbar {
    margin-left: auto;
    justify-content: flex-end;
  }

  .calendar-filters {
    display: inline-flex;
    gap: 6px;
    padding: 4px;
    border-radius: 999px;
    background: var(--acta-color-bg-subtle);
  }

  .calendar-filters button {
    border: 0;
    background: transparent;
    color: var(--acta-color-text-muted);
    padding: 8px 12px;
    border-radius: 999px;
  }

  .calendar-filters button.active {
    background: var(--acta-color-accent-soft);
    color: var(--acta-color-accent-hover);
  }

  .calendar-summary {
    flex-wrap: wrap;
  }

  .calendar-summary-card,
  .calendar-grid-panel,
  .calendar-side-panel {
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-elevated) 82%, white 18%);
  }

  .calendar-summary-card {
    min-width: 180px;
    padding: 14px 16px;
    display: grid;
    gap: 6px;
  }

  .calendar-summary-card strong {
    font-size: 1.35rem;
  }

  .calendar-summary-card span {
    color: var(--acta-color-text-muted);
  }

  .calendar-layout {
    align-items: stretch;
    flex-wrap: wrap;
  }

  .calendar-grid-panel {
    flex: 1 1 680px;
    padding: 16px;
    display: grid;
    gap: 12px;
  }

  .calendar-side-panel {
    flex: 0 0 360px;
    padding: 16px;
    display: grid;
    gap: 14px;
  }

  .calendar-weekdays,
  .calendar-grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 10px;
  }

  .calendar-weekdays span {
    color: var(--acta-color-text-muted);
    font-size: 0.85rem;
    text-align: center;
  }

  .calendar-grid:focus-visible {
    outline: 2px solid var(--acta-color-accent-hover);
    outline-offset: 6px;
    border-radius: var(--acta-radius-2xl);
  }

  .calendar-day {
    min-height: 132px;
    padding: 12px;
    border-radius: var(--acta-radius-xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
    display: grid;
    align-content: start;
    gap: 10px;
    text-align: left;
  }

  .calendar-day.is-muted {
    opacity: 0.65;
    background: var(--acta-color-bg-subtle);
  }

  .calendar-day.is-selected {
    border-color: var(--acta-color-accent-hover);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--acta-color-accent-soft) 70%, transparent 30%);
  }

  .calendar-day.has-overdue {
    background: color-mix(in srgb, var(--acta-color-bg-elevated) 86%, var(--acta-color-danger-soft) 14%);
  }

  .calendar-day-top,
  .calendar-day-totals,
  .calendar-day-events,
  .calendar-event-meta {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .calendar-day-top {
    justify-content: space-between;
    align-items: center;
  }

  .calendar-day-top strong {
    font-size: 1rem;
  }

  .calendar-day-top span,
  .calendar-pill {
    border-radius: 999px;
    padding: 4px 8px;
    font-size: 0.78rem;
  }

  .calendar-day-top span {
    background: var(--acta-color-bg-subtle);
    color: var(--acta-color-text-muted);
  }

  .calendar-money {
    font-size: 0.78rem;
    font-weight: 600;
  }

  .calendar-money.is-positive {
    color: var(--positive-strong);
  }

  .calendar-money.is-negative {
    color: var(--acta-color-danger);
  }

  .calendar-pill {
    background: var(--acta-color-bg-subtle);
    color: var(--acta-color-text);
  }

  .calendar-pill.is-schedule {
    background: color-mix(in srgb, var(--acta-color-accent-soft) 76%, white 24%);
    color: var(--acta-color-accent-hover);
  }

  .calendar-pill.is-task {
    background: color-mix(in srgb, var(--acta-color-info-soft) 76%, white 24%);
    color: var(--acta-color-info);
  }

  .calendar-pill.is-overdue {
    background: color-mix(in srgb, var(--acta-color-danger-soft) 78%, white 22%);
    color: var(--acta-color-danger);
  }

  .calendar-pill.is-done {
    background: color-mix(in srgb, var(--acta-color-success-soft) 78%, white 22%);
    color: var(--acta-color-success);
  }

  .calendar-pill.is-more {
    color: var(--acta-color-text-muted);
  }

  .calendar-event-list,
  .calendar-event-detail,
  .calendar-note {
    display: grid;
    gap: 12px;
  }

  .calendar-event-row {
    width: 100%;
    padding: 12px;
    border-radius: var(--acta-radius-xl);
    border: 1px solid var(--acta-color-border);
    background: var(--acta-color-bg-elevated);
    text-align: left;
  }

  .calendar-event-row.is-selected {
    border-color: var(--acta-color-accent-hover);
    background: color-mix(in srgb, var(--acta-color-accent-soft) 18%, var(--acta-color-bg-elevated) 82%);
  }

  .calendar-event-meta {
    justify-content: flex-end;
    color: var(--acta-color-text-muted);
    font-size: 0.85rem;
  }

  .calendar-event-facts {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .calendar-event-facts div {
    padding: 10px 12px;
    border-radius: var(--acta-radius-lg);
    background: var(--acta-color-bg-subtle);
  }

  .calendar-event-facts dt {
    color: var(--acta-color-text-muted);
    font-size: 0.8rem;
  }

  .calendar-event-facts dd {
    margin: 6px 0 0;
    font-weight: 600;
  }

  .calendar-event-actions {
    flex-wrap: wrap;
  }

  .calendar-loading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 16px;
  }

  .calendar-loading-grid,
  .calendar-loading-side {
    min-height: 320px;
    border-radius: var(--acta-radius-2xl);
    background: linear-gradient(120deg, var(--acta-color-bg-subtle), color-mix(in srgb, var(--acta-color-bg-subtle) 72%, white 28%));
  }

  @media (max-width: 960px) {
    .calendar-shell-header,
    .calendar-toolbar,
    .calendar-nav,
    .calendar-layout {
      flex-direction: column;
      align-items: stretch;
    }

    .calendar-toolbar {
      margin-left: 0;
    }

    .calendar-side-panel {
      flex-basis: auto;
    }
  }

  @media (max-width: 720px) {
    .calendar-grid,
    .calendar-weekdays {
      gap: 8px;
    }

    .calendar-day {
      min-height: 112px;
      padding: 10px;
    }

    .calendar-event-facts {
      grid-template-columns: 1fr;
    }

    .calendar-loading {
      grid-template-columns: 1fr;
    }
  }
</style>
