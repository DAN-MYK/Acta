<script lang="ts">
  import {
    PAYMENT_CALENDAR_COPY,
    CALENDAR_FILTER_OPTIONS
  } from "../config/ui";
  import PaymentCalendarHeader from "./payments-calendar/PaymentCalendarHeader.svelte";
  import PaymentCalendarSummary from "./payments-calendar/PaymentCalendarSummary.svelte";
  import {
    countEventsByKind,
    getCalendarEventDirectionLabel,
    getCalendarEventKindLabel,
    getCalendarEventTone,
    getDayAriaLabel,
    getFilteredEvents,
    getSelectedCalendarEvent,
    getVisibleEventCount,
    getWeekdayLabels
  } from "./payments-calendar/payment-calendar-view-model";
  import { paymentsStore } from "../stores/payments";

  const payments = paymentsStore;
  const calendarFilterOptions = CALENDAR_FILTER_OPTIONS;

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

  $: calendarDays = $payments.calendar?.days ?? [];
  $: weekdayLabels = getWeekdayLabels(calendarDays);
  $: selectedDay = $payments.calendar?.days.find((day) => day.selected) ?? null;
  $: selectedEvents = selectedDay ? getFilteredEvents(selectedDay, $payments.calendarFilter) : [];
  $: selectedEvent = getSelectedCalendarEvent(selectedEvents, $payments.selectedCalendarEventId);
  $: visibleEventCount = getVisibleEventCount(calendarDays, $payments.calendarFilter);
  $: scheduleCount = countEventsByKind(calendarDays, "schedule");
  $: taskCount = countEventsByKind(calendarDays, "task");
</script>

<section class="calendar-shell" data-testid="payments-calendar">
  <PaymentCalendarHeader
    monthLabel={$payments.calendar?.monthLabel ?? PAYMENT_CALENDAR_COPY.loadingMonth}
    calendarLoading={$payments.calendarLoading}
    calendarFilter={$payments.calendarFilter}
    filterOptions={calendarFilterOptions}
    onShiftMonth={(delta) => payments.shiftCalendarMonth(delta)}
    onSetFilter={(filter) => payments.setCalendarFilter(filter)}
  />

  <PaymentCalendarSummary {scheduleCount} {taskCount} {visibleEventCount} />

  {#if $payments.calendarError && !$payments.calendar}
    <div class="empty-state-card" data-testid="payments-calendar-error" role="alert">
      <strong>{PAYMENT_CALENDAR_COPY.errorTitle}</strong>
      <p>{$payments.calendarError}</p>
      <button class="btn-secondary" on:click={() => payments.loadCalendar()}>{PAYMENT_CALENDAR_COPY.retryAction}</button>
    </div>
  {:else if $payments.calendarInitialLoading && !$payments.calendar}
    <div class="calendar-loading" data-testid="payments-calendar-loading">
      <div class="calendar-loading-grid"></div>
      <div class="calendar-loading-side"></div>
    </div>
  {:else if !$payments.calendar}
    <div class="empty-state-card" data-testid="payments-calendar-empty">
      <strong>{PAYMENT_CALENDAR_COPY.emptyTitle}</strong>
      <p>{PAYMENT_CALENDAR_COPY.emptyDescription}</p>
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
              aria-label={getDayAriaLabel(day, $payments.calendarFilter)}
            >
              <div class="calendar-day-top">
                <strong>{day.dayNumber}</strong>
                {#if getFilteredEvents(day, $payments.calendarFilter).length > 0}
                  <span>{getFilteredEvents(day, $payments.calendarFilter).length}</span>
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
                {#each getFilteredEvents(day, $payments.calendarFilter).slice(0, 2) as event}
                  <span class={`calendar-pill is-${getCalendarEventTone(event)}`}>
                    {event.title}
                  </span>
                {/each}
                {#if getFilteredEvents(day, $payments.calendarFilter).length > 2}
                  <span class="calendar-pill is-more">+{getFilteredEvents(day, $payments.calendarFilter).length - 2} ще</span>
                {/if}
              </div>
            </button>
          {/each}
        </div>

        {#if visibleEventCount === 0}
          <div class="empty-state-card compact" data-testid="payments-calendar-filter-empty">
            <strong>{PAYMENT_CALENDAR_COPY.filterEmptyTitle}</strong>
            <p>{PAYMENT_CALENDAR_COPY.filterEmptyDescription}</p>
          </div>
        {/if}
      </div>

      <aside class="calendar-side-panel" data-testid="payments-calendar-details">
        <div class="calendar-side-header">
          <div>
            <strong>{selectedDay?.date ?? PAYMENT_CALENDAR_COPY.emptyDayLabel}</strong>
            <p>
              {#if selectedEvents.length > 0}
                {selectedEvents.length} подій у вибраному дні
              {:else}
                {PAYMENT_CALENDAR_COPY.emptyDayFiltered}
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
                  <span class={`task-pill is-${getCalendarEventTone(event)}`}>{getCalendarEventKindLabel(event)}</span>
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
                <span class={`task-pill is-${getCalendarEventTone(selectedEvent)}`}>{selectedEvent.statusLabel}</span>
              </div>

              <dl class="calendar-event-facts">
                <div>
                  <dt>Тип</dt>
                  <dd>{getCalendarEventKindLabel(selectedEvent)}</dd>
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
                    <dd>{getCalendarEventDirectionLabel(selectedEvent)}</dd>
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
            <strong>{PAYMENT_CALENDAR_COPY.emptyDayEvents}</strong>
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

  .calendar-layout,
  .calendar-side-header,
  .calendar-event-row,
  .calendar-event-detail-top,
  .calendar-event-actions {
    display: flex;
    gap: 12px;
  }

  .calendar-side-header,
  .calendar-event-row,
  .calendar-event-detail-top {
    justify-content: space-between;
    align-items: flex-start;
  }

  .calendar-side-header strong,
  .calendar-event-detail strong {
    margin: 0;
  }

  .calendar-side-header p,
  .calendar-event-row p,
  .calendar-event-detail p,
  .calendar-note p {
    margin: 4px 0 0;
    color: var(--acta-color-text-muted);
  }

  .calendar-grid-panel,
  .calendar-side-panel {
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-elevated) 82%, white 18%);
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
