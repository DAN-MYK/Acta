<script lang="ts">
  import { PAYMENT_CALENDAR_COPY } from "../../config/ui";
  import type { PaymentCalendarFilterKind } from "../../types";

  export let monthLabel: string = PAYMENT_CALENDAR_COPY.loadingMonth;
  export let calendarLoading = false;
  export let calendarFilter: PaymentCalendarFilterKind = "all";
  export let filterOptions: Array<{ kind: PaymentCalendarFilterKind; label: string }> = [];
  export let onShiftMonth: (delta: number) => void;
  export let onSetFilter: (filter: PaymentCalendarFilterKind) => void;
</script>

<div class="calendar-shell-header">
  <div>
    <h3>{PAYMENT_CALENDAR_COPY.title}</h3>
    <p>Місячна сітка показує планові платежі та дедлайни задач в одному часовому контексті.</p>
  </div>
  <div class="calendar-toolbar">
    <div class="calendar-nav">
      <button class="btn-ghost" on:click={() => onShiftMonth(-1)} disabled={calendarLoading}>
        {PAYMENT_CALENDAR_COPY.previousMonth}
      </button>
      <strong>{monthLabel}</strong>
      <button class="btn-ghost" on:click={() => onShiftMonth(1)} disabled={calendarLoading}>
        {PAYMENT_CALENDAR_COPY.nextMonth}
      </button>
    </div>
    <div class="calendar-filters" role="radiogroup" aria-label="Фільтр подій календаря">
      {#each filterOptions as option}
        <button
          class:active={calendarFilter === option.kind}
          on:click={() => onSetFilter(option.kind)}
          role="radio"
          aria-checked={calendarFilter === option.kind}
        >
          {option.label}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .calendar-shell-header,
  .calendar-toolbar,
  .calendar-nav {
    display: flex;
    gap: 12px;
  }

  .calendar-shell-header {
    justify-content: space-between;
    align-items: flex-start;
  }

  .calendar-shell-header h3 {
    margin: 0;
  }

  .calendar-shell-header p {
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

  @media (max-width: 1180px) {
    .calendar-shell-header {
      flex-direction: column;
    }

    .calendar-toolbar {
      margin-left: 0;
      justify-content: flex-start;
    }
  }
</style>
