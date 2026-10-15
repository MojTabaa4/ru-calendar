<script lang="ts" module>
  import type { MonthGrid } from "../lib/bindings/MonthGrid";

  /** What the header shows for each view of the popover. */
  export type Heading =
    | { mode: "days"; grid: MonthGrid }
    | { mode: "months"; yearLabel: string }
    | { mode: "years"; range: string }
    | { mode: "goto" };
</script>

<script lang="ts">
  import type { CalendarKind } from "../lib/bindings/CalendarKind";
  import { pop, timeline } from "../lib/motion";

  let {
    heading,
    direction,
    showToday,
    paging,
    onPrev,
    onNext,
    onToday,
    onPickMonth,
    onPickYear,
    onGoTo,
  }: {
    heading: Heading;
    direction: number;
    showToday: boolean;
    /** Labels for the arrows, or null to hide them. */
    paging: { prev: string; next: string } | null;
    onPrev: () => void;
    onNext: () => void;
    onToday: () => void;
    onPickMonth: () => void;
    onPickYear: () => void;
    /** Opens go-to-date in `kind`'s calendar. */
    onGoTo: (kind: CalendarKind) => void;
  } = $props();

  const key = $derived(
    heading.mode === "days"
      ? heading.grid.title
      : heading.mode === "months"
        ? `months ${heading.yearLabel}`
        : heading.mode === "years"
          ? `years ${heading.range}`
          : "goto",
  );
</script>

<header>
  <div class="titles stack">
    {#key key}
      <div in:timeline={{ direction, distance: 14 }} out:timeline={{ direction, distance: 14 }}>
        {#if heading.mode === "days"}
          <h1>
            <button class="pick" onclick={onPickMonth} title="انتخاب ماه">{heading.grid.monthName}</button>
            <button class="pick" onclick={onPickYear} title="انتخاب سال">{heading.grid.yearLabel}</button>
          </h1>
          <!-- Each calendar's line opens go-to-date in that calendar. -->
          <p>
            <button class="line" onclick={() => onGoTo("gregorian")} title="رفتن به تاریخ میلادی">
              {heading.grid.gregorianSpan}
              {#if heading.grid.gregorianSpanEn}<span class="en" dir="ltr">{heading.grid.gregorianSpanEn}</span>{/if}
            </button>
          </p>
          <p>
            <button class="line" onclick={() => onGoTo("hijri")} title="رفتن به تاریخ قمری">
              {heading.grid.hijriSpan}
            </button>
          </p>
        {:else if heading.mode === "months"}
          <h1><button class="pick" onclick={onPickYear} title="انتخاب سال">{heading.yearLabel}</button></h1>
          <p>ماه را انتخاب کنید</p>
          <p>&nbsp;</p>
        {:else if heading.mode === "years"}
          <h1>{heading.range}</h1>
          <p>سال را انتخاب کنید</p>
          <p>&nbsp;</p>
        {:else}
          <h1>رفتن به تاریخ</h1>
          <p>خورشیدی، میلادی یا قمری</p>
          <p>&nbsp;</p>
        {/if}
      </div>
    {/key}
  </div>
  <nav>
    {#if showToday}
      <button class="today" onclick={onToday} transition:pop>امروز</button>
    {/if}
    <button
      class="icon"
      class:on={heading.mode === "goto"}
      aria-label="رفتن به تاریخ"
      title="رفتن به تاریخ (⌘G)"
      aria-pressed={heading.mode === "goto"}
      onclick={() => onGoTo("persian")}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <rect x="2" y="3" width="12" height="11" rx="2" />
        <path d="M2 6.5h12M5.5 1.5v3M10.5 1.5v3M9 10.5h3m-1.5-1.5 1.5 1.5-1.5 1.5" />
      </svg>
    </button>
    {#if paging}
      <!-- In RTL the past is to the right, so "previous" points right. -->
      <button class="icon" aria-label={paging.prev} title={paging.prev} onclick={onPrev}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3l5 5-5 5" /></svg>
      </button>
      <button class="icon" aria-label={paging.next} title={paging.next} onclick={onNext}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
      </button>
    {/if}
  </nav>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-inline: 4px;
  }

  .titles {
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 19px;
    font-weight: 700;
    line-height: 1.3;
    display: flex;
    gap: 4px;
  }

  /* The month and the year open their pickers. */
  .pick {
    font-weight: 700;
    padding: 0 4px;
    margin-inline: -4px 0;
    border-radius: var(--radius-sm);
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }

  .pick:hover {
    background: var(--hover);
    color: var(--accent);
  }

  .pick:active {
    transform: scale(0.96);
  }

  p {
    margin: 0;
    font-size: 11px;
    line-height: 1.45;
    color: var(--muted);
    white-space: nowrap;
  }

  .line {
    font-size: inherit;
    color: inherit;
    padding: 0 4px;
    margin-inline: -4px 0;
    border-radius: 5px;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }

  .line:hover {
    background: var(--hover);
    color: var(--accent);
  }

  .line:hover .en {
    color: inherit;
  }

  .line:active {
    transform: scale(0.97);
  }

  .en {
    margin-inline-start: 6px;
    color: var(--faint);
    font-size: 10.5px;
  }

  nav {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .icon {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    display: grid;
    place-items: center;
  }

  .icon,
  .today {
    transition:
      background-color 120ms ease,
      color 120ms ease,
      transform 90ms ease;
  }

  .icon:hover,
  .today:hover {
    background: var(--hover);
  }

  .icon.on {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .icon:active,
  .today:active {
    transform: scale(0.9);
  }

  svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .today {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    padding: 3px 10px;
    border-radius: var(--radius-sm);
  }
</style>
