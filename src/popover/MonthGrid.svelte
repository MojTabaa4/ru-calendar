<script lang="ts">
  import type { EventView } from "../lib/bindings/EventView";
  import type { MonthGrid } from "../lib/bindings/MonthGrid";
  import { untrack } from "svelte";
  import { timeline } from "../lib/motion";

  let {
    grid,
    direction,
    events,
    selected,
    onSelect,
    element = $bindable(null),
  }: {
    grid: MonthGrid;
    /** 1 if this month is later than the previous one shown, -1 if earlier. */
    direction: number;
    events: EventView[];
    selected: number | null;
    onSelect: (jdn: number) => void;
    element?: HTMLElement | null;
  } = $props();

  // Up to three colours per day, one per calendar.
  const dots = $derived.by(() => {
    const byDay = new Map<number, string[]>();
    for (const event of events) {
      for (let jdn = event.startJdn; jdn <= event.endJdn; jdn++) {
        const colors = byDay.get(jdn) ?? [];
        if (colors.length < 3 && !colors.includes(event.color)) colors.push(event.color);
        byDay.set(jdn, colors);
      }
    }
    return byDay;
  });

  // Where the selection highlight sits; it glides between days of the same month.
  const selectedIndex = $derived(grid.cells.findIndex((c) => c.jdn === selected));
  // While the selected day isn't in view, the highlight fades out where it was.
  let place = $state(untrack(() => Math.max(selectedIndex, 0)));
  $effect(() => {
    if (selectedIndex >= 0) place = selectedIndex;
  });
</script>

<div class="weekdays" aria-hidden="true">
  {#each grid.weekdays as weekday (weekday.name)}
    <span class:weekend={weekday.weekend} title={weekday.name}>{weekday.short}</span>
  {/each}
</div>

<div class="viewport stack">
  {#key grid.year * 12 + grid.month}
    <div
      class="grid"
      role="grid"
      aria-label={grid.title}
      tabindex="0"
      bind:this={element}
      in:timeline={{ direction }}
      out:timeline={{ direction }}
    >
      <span
        class="selection"
        class:shown={selectedIndex >= 0}
        style:--col={place % 7}
        style:--row={Math.floor(place / 7)}
        aria-hidden="true"
      ></span>
      {#each grid.cells as cell (cell.jdn)}
        <button
          class="cell"
          class:out={!cell.inMonth}
          class:today={cell.isToday}
          class:holiday={cell.status !== "normal"}
          class:mourning={cell.mourning}
          tabindex="-1"
          aria-pressed={cell.jdn === selected}
          onclick={() => onSelect(cell.jdn)}
        >
          <span class="num">{cell.label}</span>
          <span class="subs">
            <span>{cell.hijriLabel ?? ""}</span>
            <!-- A ring marks where a Gregorian month begins. -->
            <span
              class:start={cell.gregorianMonthStart}
              title={cell.gregorianMonthStart ? `آغاز ${cell.gregorianMonthStart}` : undefined}
              >{cell.gregorianLabel ?? ""}</span
            >
          </span>
          {#if cell.hasObservance}
            <span class="dot" aria-hidden="true"></span>
          {/if}
          {#if dots.get(cell.jdn)}
            <span class="events" aria-hidden="true">
              {#each dots.get(cell.jdn)! as color (color)}
                <span style:background={color}></span>
              {/each}
            </span>
          {/if}
        </button>
      {/each}
    </div>
  {/key}
</div>

<style>
  .weekdays,
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
  }

  .weekdays span {
    text-align: center;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    padding-bottom: 2px;
  }

  .weekdays .weekend {
    color: var(--holiday);
  }

  /* Clips the months sliding past each other. */
  .viewport {
    overflow: hidden;
  }

  .grid {
    --gap: 2px;
    --cell-height: 44px;
    position: relative;
    gap: var(--gap);
    outline: none;
  }

  /* Sits over the selected day (translucent, so holiday colours show through) and
     glides to the next one. */
  .selection {
    position: absolute;
    z-index: 1;
    pointer-events: none;
    top: calc(var(--row) * (var(--cell-height) + var(--gap)));
    inset-inline-start: calc(var(--col) * (100% + var(--gap)) / 7);
    width: calc((100% - 6 * var(--gap)) / 7);
    height: var(--cell-height);
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    opacity: 0;
    transition:
      top 220ms cubic-bezier(0.2, 0.8, 0.2, 1),
      inset-inline-start 220ms cubic-bezier(0.2, 0.8, 0.2, 1),
      opacity 150ms ease;
  }

  .selection.shown {
    opacity: 1;
  }

  .cell {
    position: relative;
    height: var(--cell-height);
    transition:
      background-color 120ms ease,
      transform 90ms ease;
    border-radius: var(--radius-sm);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    line-height: 1.1;
  }

  .cell:hover {
    background: var(--hover);
  }

  .cell:active {
    transform: scale(0.94);
  }

  .num {
    font-size: 16px;
    font-weight: 600;
  }

  .subs {
    display: flex;
    justify-content: space-between;
    width: 100%;
    padding-inline: 6px;
    font-size: 9.5px;
    color: var(--muted);
    margin-top: 2px;
  }

  .subs .start {
    display: inline-grid;
    place-items: center;
    min-width: 13px;
    height: 13px;
    padding: 0 2px;
    margin: -1px -2px;
    border: 1px solid currentColor;
    border-radius: 999px;
    line-height: 1;
  }

  .holiday .num {
    color: var(--holiday);
  }

  .holiday:not(.out) {
    background: var(--holiday-soft);
  }

  .holiday:not(.out):hover {
    background: var(--hover);
  }

  .out {
    opacity: 0.35;
  }

  .today {
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }


  .dot,
  .events span {
    animation: dot-in 260ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  @keyframes dot-in {
    from {
      transform: scale(0);
      opacity: 0;
    }
  }

  .dot {
    position: absolute;
    top: 5px;
    inset-inline-start: 6px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent);
  }

  .events {
    position: absolute;
    top: 5px;
    inset-inline-end: 5px;
    display: flex;
    gap: 2px;
  }

  .events span {
    width: 4px;
    height: 4px;
    border-radius: 50%;
  }

  .holiday .dot {
    background: var(--holiday);
  }

  .mourning .dot {
    background: var(--mourning);
  }
</style>
