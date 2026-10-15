<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import type { PrayerDayView } from "../lib/bindings/PrayerDayView";
  import { onAppEvent } from "../lib/events";
  import { rise } from "../lib/motion";

  let { jdn }: { jdn: number | null } = $props();

  let view = $state<PrayerDayView | null>(null);
  // The day `view` belongs to; times fade in when it changes, not on every refresh.
  let shownFor = $state<number | null>(null);
  let latest = 0;

  async function load(day: number | null) {
    const request = ++latest;
    try {
      const next = await api.prayerTimes(day ?? undefined);
      if (request === latest) {
        view = next;
        shownFor = day;
      }
    } catch {
      if (request === latest) view = null;
    }
  }

  $effect(() => {
    void load(jdn);
  });

  onMount(() => {
    // Keeps "next" and the countdown current while the popover stays open.
    const timer = setInterval(() => void load(jdn), 30_000);
    const stop = onAppEvent("settings-changed", () => void load(jdn));
    return () => {
      clearInterval(timer);
      stop();
    };
  });
</script>

{#if view}
  <section aria-label="اوقات شرعی">
    <header>
      <span>به افق {view.place}</span>
      {#if view.countdown}<span class="countdown">{view.countdown}</span>{/if}
    </header>
    {#key shownFor}
    <ol in:rise={{ y: 3 }}>
      {#each view.times as time (time.prayer)}
        <li class:passed={time.passed} class:next={time.next} title={time.name}>
          <span class="name">{time.short}</span>
          <span class="time">{time.time}</span>
        </li>
      {/each}
    </ol>
    {/key}
  </section>
{/if}

<style>
  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 6px 8px 8px;
  }

  header {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    color: var(--muted);
    padding-inline: 2px;
    margin-bottom: 4px;
  }

  .countdown {
    color: var(--accent);
    white-space: nowrap;
  }

  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 2px;
  }

  li {
    display: flex;
    flex-direction: column;
    align-items: center;
    border-radius: var(--radius-sm);
    padding: 3px 0;
    line-height: 1.35;
    transition: background-color 400ms ease;
  }

  .name,
  .time {
    transition: color 400ms ease;
  }

  .name {
    font-size: 10.5px;
    color: var(--muted);
  }

  .time {
    font-size: 12.5px;
    font-weight: 600;
  }

  .passed .time {
    color: var(--faint);
  }

  .next {
    background: var(--accent-soft);
  }

  .next .name,
  .next .time {
    color: var(--accent);
  }
</style>
