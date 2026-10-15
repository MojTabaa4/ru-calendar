<script lang="ts">
  import type { MonthsView } from "../lib/bindings/MonthsView";
  import { parseNumber } from "../lib/digits";

  let {
    label,
    date,
    months,
    onChange,
  }: {
    label: string;
    date: { year: number; month: number; day: number };
    months: MonthsView;
    /** Resolves to whether the date could be converted. */
    onChange: (year: number, month: number, day: number) => Promise<boolean>;
  } = $props();

  // What's in the year box while typing; reset whenever a conversion lands.
  let yearText = $state("");
  $effect(() => {
    yearText = months.yearLabel;
  });

  async function commitYear() {
    const year = parseNumber(yearText);
    if (Number.isNaN(year) || (year !== date.year && !(await onChange(year, date.month, date.day)))) {
      yearText = months.yearLabel;
    }
  }

  // Enter needs no handler: it fires the input's change event, which commits.
  function onYearKey(event: KeyboardEvent) {
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      void onChange(date.year + (event.key === "ArrowUp" ? 1 : -1), date.month, date.day);
    }
  }
</script>

<div class="row">
  <span class="label">{label}</span>
  <!-- Day, month, year: read right to left as "۲۳ مهر ۱۴۰۵". -->
  <select
    class="day"
    aria-label="{label}، روز"
    value={date.day}
    onchange={(e) => onChange(date.year, date.month, Number(e.currentTarget.value))}
  >
    {#each { length: months.lengths[date.month - 1] } as _, i (i)}
      <option value={i + 1}>{months.dayLabels[i]}</option>
    {/each}
  </select>
  <select
    class="month"
    aria-label="{label}، ماه"
    value={date.month}
    onchange={(e) => onChange(date.year, Number(e.currentTarget.value), date.day)}
  >
    {#each months.names as name, i (i)}
      <option value={i + 1}>{months.altNames[i] ? `${name} · ${months.altNames[i]}` : name}</option>
    {/each}
  </select>
  <input
    class="year"
    aria-label="{label}، سال"
    inputmode="numeric"
    bind:value={yearText}
    onchange={commitYear}
    onkeydown={onYearKey}
  />
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 64px 64px 1fr 80px;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
  }

  .label {
    font-size: 13px;
    font-weight: 600;
  }

  select,
  input {
    font: inherit;
    font-size: 13px;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    height: 30px;
    padding: 0 8px;
    min-width: 0;
  }

  select:focus-visible,
  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .year {
    text-align: center;
  }
</style>
