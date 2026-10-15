<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api } from "../lib/api";
  import type { CalendarKind } from "../lib/bindings/CalendarKind";
  import type { DayDetails } from "../lib/bindings/DayDetails";
  import type { MonthsView } from "../lib/bindings/MonthsView";
  import Segmented from "../lib/controls/Segmented.svelte";
  import DateRow from "../converter/DateRow.svelte";

  let {
    from,
    kind: startKind,
    onGo,
  }: { from: number; kind: CalendarKind; onGo: (jdn: number) => void } = $props();

  const labels: Record<CalendarKind, string> = { persian: "خورشیدی", gregorian: "میلادی", hijri: "قمری" };

  let kind = $state<CalendarKind>(untrack(() => startKind));
  let target = $state<DayDetails | null>(null);
  let months = $state<MonthsView | null>(null);
  let error = $state<string | null>(null);

  function dateIn(day: DayDetails, k: CalendarKind) {
    return k === "persian" ? day.persianDate : k === "gregorian" ? day.gregorianDate : day.hijriDate;
  }

  async function show(day: DayDetails, k: CalendarKind) {
    months = await api.converterMonths(k, dateIn(day, k).year);
    target = day;
    kind = k;
    error = null;
  }

  // Keeps the date in range: a day past the month's end becomes its last day.
  async function change(year: number, month: number, day: number): Promise<boolean> {
    try {
      const view = await api.converterMonths(kind, year);
      await show(await api.convertDate(kind, year, month, Math.min(day, view.lengths[month - 1])), kind);
      return true;
    } catch {
      error = "این تاریخ وجود ندارد.";
      return false;
    }
  }

  function go() {
    if (target) onGo(target.jdn);
  }

  onMount(() => {
    void api.day(from).then((day) => show(day, kind));
  });
</script>

<form
  class="goto"
  onsubmit={(e) => {
    e.preventDefault();
    go();
  }}
>
  <div class="kinds">
    <Segmented
      label="تقویم"
      options={(["persian", "gregorian", "hijri"] as const).map((k) => ({ value: k, label: labels[k] }))}
      value={kind}
      onchange={(k) => target && void show(target, k)}
    />
  </div>
  {#if target && months}
    <div class="row">
      <DateRow label={labels[kind]} date={dateIn(target, kind)} {months} onChange={change} />
    </div>
    <p class="preview" class:holiday={target.status !== "normal"}>{target.weekday} {target.persian}</p>
    <p class="secondary">{target.gregorian} <span class="sep">|</span> {target.hijri}</p>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <button type="submit" class="go" disabled={!target}>برو به این روز</button>
</form>

<style>
  .goto {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 6px 2px;
  }

  .kinds {
    display: flex;
    justify-content: center;
  }

  .row {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  /* The switch above already names the calendar, so the row skips its label and
     gives the month (e.g. "اکتبر · October") the room. */
  .row :global(.row) {
    grid-template-columns: 58px 1fr 66px;
    padding: 8px 10px;
  }

  .row :global(.label) {
    display: none;
  }

  .preview {
    margin: 4px 4px 0;
    font-size: 15px;
    font-weight: 700;
  }

  .preview.holiday {
    color: var(--holiday);
  }

  .secondary {
    margin: 0 4px;
    font-size: 12px;
    color: var(--muted);
  }

  .sep {
    margin-inline: 4px;
    color: var(--faint);
  }

  .error {
    margin: 0 4px;
    font-size: 12px;
    color: var(--holiday);
  }

  .go {
    margin-top: auto;
    align-self: stretch;
    padding: 8px;
    border-radius: var(--radius);
    background: var(--accent);
    color: #fff;
    font-weight: 600;
    transition:
      opacity 120ms ease,
      transform 90ms ease;
  }

  .go:active:not(:disabled) {
    transform: scale(0.97);
  }

  .go:disabled {
    opacity: 0.5;
  }
</style>
