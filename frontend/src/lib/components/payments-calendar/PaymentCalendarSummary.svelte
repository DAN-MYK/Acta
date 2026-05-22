<script lang="ts">
  import { PAYMENT_CALENDAR_COPY } from "../../config/ui";

  export let scheduleCount = 0;
  export let taskCount = 0;
  export let visibleEventCount = 0;
</script>

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
    <span>{PAYMENT_CALENDAR_COPY.visibleEventsSummary}</span>
  </div>
</div>

<style>
  .calendar-summary {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }

  .calendar-summary-card {
    min-width: 180px;
    padding: 14px 16px;
    display: grid;
    gap: 6px;
    border-radius: var(--acta-radius-2xl);
    border: 1px solid var(--acta-color-border);
    background: color-mix(in srgb, var(--acta-color-bg-elevated) 82%, white 18%);
  }

  .calendar-summary-card strong {
    font-size: 1.35rem;
  }

  .calendar-summary-card span {
    color: var(--acta-color-text-muted);
  }
</style>
