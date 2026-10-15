<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { onAppEvent } from "../lib/events";
  import type { CalendarKind } from "../lib/bindings/CalendarKind";
  import type { Digits } from "../lib/bindings/Digits";
  import { localDigits } from "../lib/digits";
  import { monthDirection, timeline, zoom } from "../lib/motion";
  import GoToDate from "./GoToDate.svelte";
  import MonthPicker from "./MonthPicker.svelte";
  import YearPicker from "./YearPicker.svelte";
  import type { Heading } from "./MonthHeader.svelte";
  import type { DayDetails as Details } from "../lib/bindings/DayDetails";
  import type { EventView } from "../lib/bindings/EventView";
  import type { MonthGrid as Grid } from "../lib/bindings/MonthGrid";
  import DayDetails from "./DayDetails.svelte";
  import Footer from "./Footer.svelte";
  import MonthGrid from "./MonthGrid.svelte";
  import MonthHeader from "./MonthHeader.svelte";
  import PrayerStrip from "./PrayerStrip.svelte";

  let grid = $state<Grid | null>(null);
  // Which way the last month change went (1 later, -1 earlier), for the slide.
  let direction = $state(0);
  // The month being shown or loading. Updated immediately on navigation, so quick
  // repeated clicks step through months instead of reloading the same one.
  let view = $state<{ year: number; month: number } | null>(null);
  let details = $state<Details | null>(null);
  // System calendar events for the six weeks in the grid.
  let events = $state<EventView[]>([]);
  let eventsRequest = 0;
  let selected = $state<number | null>(null);
  let todayJdn = $state<number | null>(null);
  let error = $state<string | null>(null);
  let gridElement = $state<HTMLElement | null>(null);

  // Responses can arrive out of order when clicking quickly; only the latest request wins.
  let latest = 0;

  function setGrid(next: Grid) {
    direction = monthDirection(grid, next);
    grid = next;
  }

  async function showMonth(year: number, month: number) {
    view = { year, month };
    const request = ++latest;
    try {
      const next = await api.month(year, month);
      if (request === latest) setGrid(next);
    } catch (e) {
      error = String(e);
    }
  }

  async function select(jdn: number) {
    const request = ++latest;
    try {
      const day = await api.day(jdn);
      if (request !== latest) return;
      selected = jdn;
      details = day;
      const { year, month } = day.persianDate;
      view = { year, month };
      if (!grid || grid.year !== year || grid.month !== month) {
        const next = await api.month(year, month);
        if (request === latest) setGrid(next);
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function goToday() {
    try {
      const today = await api.today();
      todayJdn = today.jdn;
      await select(today.jdn);
    } catch (e) {
      error = String(e);
    }
  }

  async function loadEvents(shown: Grid) {
    const request = ++eventsRequest;
    const first = shown.cells[0].jdn;
    const last = shown.cells[shown.cells.length - 1].jdn;
    try {
      const next = await api.events(first, last);
      if (request === eventsRequest) events = next;
    } catch {
      if (request === eventsRequest) events = [];
    }
  }

  $effect(() => {
    if (grid) void loadEvents(grid);
  });

  const selectedEvents = $derived(
    selected === null ? [] : events.filter((e) => e.startJdn <= selected! && selected! <= e.endJdn),
  );

  // Settings change digits, names and which days are marked, so reload what's shown.
  async function reload() {
    // select() moves the view to the selected day's month; keep the one being looked at.
    const shown = view;
    if (selected !== null) {
      await select(selected);
    }
    if (shown) {
      await showMonth(shown.year, shown.month);
    }
  }

  function shiftMonth(delta: number) {
    if (!view) return;
    const index = view.year * 12 + view.month - 1 + delta;
    void showMonth(Math.floor(index / 12), (((index % 12) + 12) % 12) + 1);
  }

  // In RTL, the next day is to the left.
  const moves: Record<string, number> = { ArrowLeft: 1, ArrowRight: -1, ArrowDown: 7, ArrowUp: -7 };

  function onKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "g") {
      event.preventDefault();
      toggleGoTo();
    } else if (event.key === "Escape") {
      // Back out of a picker first; Escape on the days closes the popover.
      if (mode !== "days") setMode("days");
      else void api.hidePopover();
    } else if (mode !== "days") {
      return;
    } else if (event.key in moves && selected !== null) {
      event.preventDefault();
      void select(selected + moves[event.key]);
    } else if (event.key === "PageDown") {
      event.preventDefault();
      shiftMonth(1);
    } else if (event.key === "PageUp") {
      event.preventDefault();
      shiftMonth(-1);
    } else if (event.key === "Home") {
      event.preventDefault();
      void goToday();
    }
  }

  onMount(() => {
    void goToday();
    void loadDisplay();
    const stops = [
      onAppEvent("popover-shown", () => {
        mode = "days";
        void goToday();
        gridElement?.focus();
      }),
      onAppEvent("day-changed", () => {
        void goToday();
        void loadDisplay();
      }),
      onAppEvent("settings-changed", () => {
        void reload();
        void loadDisplay();
      }),
      onAppEvent("calendar-store-changed", () => grid && void loadEvents(grid)),
    ];
    return () => stops.forEach((stop) => stop());
  });

  // The area under the header shows days, or zooms out to months or years, or the
  // go-to-date form.
  type Mode = "days" | "months" | "years" | "goto";
  const LEVEL: Record<Mode, number> = { days: 0, months: 1, goto: 1, years: 2 };

  let mode = $state<Mode>("days");
  let pickerYear = $state(0);
  let yearStart = $state(0);
  let monthNames = $state<string[]>([]);
  let digits = $state<Digits>("persian");
  let todayYm = $state<{ year: number; month: number } | null>(null);
  // Where to go after picking a year: straight to that year's month, or to its months.
  let yearsReturnTo = $state<"days" | "months">("days");
  // How the body changes next: zooming between levels or sliding within one. Read by
  // the transitions as they start, so it's set just before the change.
  let motion = $state<{ kind: "zoom" | "slide"; step: number }>({ kind: "zoom", step: 0 });

  function setMode(next: Mode) {
    motion = { kind: "zoom", step: Math.sign(LEVEL[next] - LEVEL[mode]) };
    mode = next;
  }

  const bodyIn = (node: Element, _: unknown, phase?: { direction?: "in" | "out" | "both" }) =>
    motion.kind === "zoom" ? zoom(node, { level: motion.step }, phase) : timeline(node, { direction: motion.step }, phase);

  async function loadDisplay() {
    const [settings, today] = await Promise.all([api.settings(), api.today()]);
    digits = settings.digits;
    todayYm = { year: today.persian.year, month: today.persian.month };
    monthNames = (await api.converterMonths("persian", today.persian.year)).names;
  }

  function openMonths() {
    pickerYear = view?.year ?? grid?.year ?? 0;
    setMode("months");
  }

  function openYears(returnTo: "days" | "months") {
    yearsReturnTo = returnTo;
    const year = returnTo === "months" ? pickerYear : (view?.year ?? grid?.year ?? 0);
    yearStart = year - 4;
    setMode("years");
  }

  function pickMonth(month: number) {
    setMode("days");
    void showMonth(pickerYear, month);
  }

  function pickYear(year: number) {
    if (yearsReturnTo === "months") {
      pickerYear = year;
      setMode("months");
    } else {
      setMode("days");
      void showMonth(year, view?.month ?? 1);
    }
  }

  function goTo(jdn: number) {
    setMode("days");
    void select(jdn);
  }

  // Which calendar go-to-date starts in, and on which day: the selected one if it's
  // in the month being viewed, otherwise that month's first day.
  let gotoKind = $state<CalendarKind>("persian");
  const gotoFrom = $derived.by(() => {
    if (!grid) return todayJdn ?? 0;
    const inMonth = grid.cells.filter((c) => c.inMonth);
    return inMonth.some((c) => c.jdn === selected) ? selected! : inMonth[0].jdn;
  });

  function toggleGoTo(kind: CalendarKind = "persian") {
    if (mode === "goto") {
      setMode("days");
    } else {
      gotoKind = kind;
      setMode("goto");
    }
  }

  // The arrows page through whatever is shown.
  function page(delta: number) {
    if (mode === "months") {
      motion = { kind: "slide", step: delta };
      pickerYear += delta;
    } else if (mode === "years") {
      motion = { kind: "slide", step: delta };
      yearStart += delta * 12;
    } else {
      shiftMonth(delta);
    }
  }

  const heading = $derived.by((): Heading | null => {
    if (mode === "months") return { mode, yearLabel: localDigits(pickerYear, digits) };
    if (mode === "years") {
      return { mode, range: `${localDigits(yearStart, digits)} – ${localDigits(yearStart + 11, digits)}` };
    }
    if (mode === "goto") return { mode };
    return grid ? { mode: "days", grid } : null;
  });

  const paging = $derived(
    mode === "months"
      ? { prev: "سال قبل", next: "سال بعد" }
      : mode === "years"
        ? { prev: "سال‌های قبل", next: "سال‌های بعد" }
        : mode === "goto"
          ? null
          : { prev: "ماه قبل", next: "ماه بعد" },
  );

  const bodyKey = $derived(mode === "months" ? `months ${pickerYear}` : mode === "years" ? `years ${yearStart}` : mode);

  const showTodayButton = $derived(
    grid !== null && (mode !== "days" || !grid.containsToday || selected !== todayJdn),
  );
</script>

<svelte:window onkeydown={onKeydown} />

<main>
  {#if grid && heading}
    <MonthHeader
      {heading}
      direction={mode === "days" ? direction : motion.kind === "slide" ? motion.step : 0}
      showToday={showTodayButton}
      {paging}
      onPrev={() => page(-1)}
      onNext={() => page(1)}
      onToday={() => {
        setMode("days");
        void goToday();
      }}
      onPickMonth={openMonths}
      onPickYear={() => openYears(mode === "months" ? "months" : "days")}
      onGoTo={toggleGoTo}
    />
    <div class="body stack">
      {#key bodyKey}
        <div class="level" in:bodyIn out:bodyIn>
          {#if mode === "days"}
            <MonthGrid {grid} {direction} {events} {selected} onSelect={select} bind:element={gridElement} />
          {:else if mode === "months"}
            <MonthPicker year={pickerYear} names={monthNames} viewed={view} today={todayYm} onPick={pickMonth} />
          {:else if mode === "years"}
            <YearPicker
              start={yearStart}
              viewed={yearsReturnTo === "months" ? pickerYear : (view?.year ?? null)}
              today={todayYm?.year ?? null}
              {digits}
              onPick={pickYear}
            />
          {:else}
            <GoToDate from={gotoFrom} kind={gotoKind} onGo={goTo} />
          {/if}
        </div>
      {/key}
    </div>
    {#if details}
      <DayDetails {details} events={selectedEvents} />
    {/if}
    <PrayerStrip jdn={selected} />
  {:else if error}
    <p class="error">{error}</p>
  {/if}
  <Footer {selected} />
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 12px 12px 8px;
    gap: 8px;
  }

  /* The days, a picker or the go-to form, all in the same space. */
  .body {
    height: 296px;
    flex-shrink: 0;
  }

  .level {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .error {
    color: var(--holiday);
    flex: 1;
    white-space: pre-wrap;
    direction: ltr;
    font-size: 12px;
  }
</style>
