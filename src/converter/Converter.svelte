<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import type { CalendarKind } from "../lib/bindings/CalendarKind";
  import type { DayDetails } from "../lib/bindings/DayDetails";
  import type { MonthsView } from "../lib/bindings/MonthsView";
  import { onAppEvent } from "../lib/events";
  import { rise } from "../lib/motion";
  import DateRow from "./DateRow.svelte";

  const rows: { kind: CalendarKind; label: string }[] = [
    { kind: "persian", label: "خورشیدی" },
    { kind: "gregorian", label: "میلادی" },
    { kind: "hijri", label: "قمری" },
  ];

  let details = $state<DayDetails | null>(null);
  let months = $state<Record<CalendarKind, MonthsView> | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);
  let todayJdn = $state<number | null>(null);
  let latest = 0;

  function dateIn(day: DayDetails, kind: CalendarKind) {
    return kind === "persian" ? day.persianDate : kind === "gregorian" ? day.gregorianDate : day.hijriDate;
  }

  // Shows `next` in all three rows once each row's month names and lengths are loaded.
  async function show(next: DayDetails, request: number) {
    const views = await Promise.all(rows.map((r) => api.converterMonths(r.kind, dateIn(next, r.kind).year)));
    if (request !== latest) return;
    months = { persian: views[0], gregorian: views[1], hijri: views[2] };
    details = next;
    error = null;
  }

  async function change(kind: CalendarKind, year: number, month: number, day: number): Promise<boolean> {
    const request = ++latest;
    try {
      // Also checks the year converts; a day past the month's end moves to its last day.
      const view = await api.converterMonths(kind, year);
      const next = await api.convertDate(kind, year, month, Math.min(day, view.lengths[month - 1]));
      await show(next, request);
      return true;
    } catch {
      if (request === latest) error = "این تاریخ را نمی‌توان تبدیل کرد.";
      return false;
    }
  }

  async function showDay(jdn: number) {
    const request = ++latest;
    try {
      await show(await api.day(jdn), request);
    } catch (e) {
      if (request === latest) error = String(e);
    }
  }

  async function goToday() {
    const today = await api.today();
    todayJdn = today.jdn;
    await showDay(today.jdn);
  }

  async function copy() {
    if (!details) return;
    const text = [`${details.weekday} ${details.persian}`, details.gregorian, details.hijri].join("\n");
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      error = "کپی نشد.";
    }
  }

  onMount(() => {
    const start = Number(new URLSearchParams(location.search).get("jdn"));
    void api.today().then((t) => (todayJdn = t.jdn));
    void (start ? showDay(start) : goToday());
    const stops = [
      onAppEvent<number>("converter-show-day", (jdn) => void showDay(jdn)),
      onAppEvent("settings-changed", () => details && void showDay(details.jdn)),
    ];
    return () => stops.forEach((stop) => stop());
  });
</script>

<main>
  {#if details && months}
    <section class="rows">
      {#each rows as row (row.kind)}
        <DateRow
          label={row.label}
          date={dateIn(details, row.kind)}
          months={months[row.kind]}
          onChange={(y, m, d) => change(row.kind, y, m, d)}
        />
      {/each}
    </section>

    <section class="result stack" aria-live="polite">
      {#key details.jdn}
      <div in:rise>
      <h1 class:holiday={details.status !== "normal"}>{details.weekday} {details.persian}</h1>
      <p class="secondary">
        <span>{details.gregorian}</span>
        <span class="sep" aria-hidden="true">|</span>
        <span>{details.hijri}</span>
      </p>
      {#if details.observances.length > 0}
        <ul>
          {#each details.observances as observance (observance.id)}
            <li class:holiday={observance.dayOff}>{observance.title}</li>
          {/each}
        </ul>
      {/if}
      {#if details.hijriApproximate}
        <p class="note">
          ≈ تاریخ قمری این روز محاسبه‌ای است و ممکن است یک یا دو روز با رؤیت هلال در ایران فرق کند.
        </p>
      {/if}
      </div>
      {/key}
    </section>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <footer>
    <button onclick={goToday} disabled={details?.jdn === todayJdn}>امروز</button>
    <button onclick={copy} disabled={!details}>{copied ? "کپی شد" : "کپی تاریخ‌ها"}</button>
  </footer>
</main>

<style>
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
  }

  .rows {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 4px 0;
  }

  .result {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-inline: 4px;
  }

  h1 {
    margin: 0;
    font-size: 17px;
    font-weight: 700;
  }

  h1.holiday {
    color: var(--holiday);
  }

  .secondary {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--muted);
  }

  .sep {
    margin-inline: 6px;
    color: var(--faint);
  }

  ul {
    margin: 10px 0 0;
    padding-inline-start: 18px;
    font-size: 12.5px;
  }

  li.holiday {
    color: var(--holiday);
  }

  .note {
    margin: 10px 0 0;
    font-size: 11.5px;
    color: var(--muted);
  }

  .error {
    margin: 0;
    font-size: 12px;
    color: var(--holiday);
  }

  footer {
    display: flex;
    justify-content: flex-start;
    gap: 8px;
  }

  footer button {
    font-size: 12.5px;
    padding: 5px 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  footer button {
    transition:
      background-color 120ms ease,
      transform 90ms ease;
  }

  footer button:hover:not(:disabled) {
    background: var(--hover);
  }

  footer button:active:not(:disabled) {
    transform: scale(0.95);
  }

  footer button:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
