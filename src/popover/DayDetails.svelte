<script lang="ts">
  import { api } from "../lib/api";
  import type { DayDetails } from "../lib/bindings/DayDetails";
  import type { EventView } from "../lib/bindings/EventView";
  import { rise } from "../lib/motion";

  let { details, events }: { details: DayDetails; events: EventView[] } = $props();
</script>

{#snippet row(event: EventView)}
  <span class="when">
    {#if event.birthday}🎂{:else if event.time && event.startJdn === details.jdn}{event.time}{:else if event.allDay}تمام روز{:else}ادامه{/if}
  </span>
  <span class="what">
    <span class="title">{event.title}</span>
    {#if event.location}<span class="where">{event.location}</span>{/if}
  </span>
{/snippet}

<section aria-live="polite">
  {#key details.jdn}
  <div in:rise>
  <h2 class:holiday={details.status !== "normal"}>{details.weekday} {details.persian}</h2>
  <p class="secondary">
    <span>{details.gregorian}</span>
    {#if details.gregorianEn}<span class="en" dir="ltr">({details.gregorianEn})</span>{/if}
    <span class="sep" aria-hidden="true">|</span>
    <span>{details.hijri}</span>
    {#if details.hijriApproximate}
      <span class="approx" title="تاریخ قمری محاسبه‌ای است و ممکن است با تقویم رسمی یک روز فرق کند">≈</span>
    {/if}
  </p>
  {#if details.observances.length > 0}
    <ul>
      {#each details.observances as observance (observance.id)}
        <li class:holiday={observance.dayOff} class:mourning={observance.mourning}>
          <span>{observance.title}</span>
          {#if observance.dayOff}
            <span class="tag">تعطیل</span>
          {/if}
        </li>
      {/each}
    </ul>
  {:else if events.length === 0}
    <p class="empty">مناسبتی برای این روز ثبت نشده است.</p>
  {/if}
  {#if events.length > 0}
    <ul class="events">
      {#each events as event (event.id)}
        <li style:--color={event.color}>
          {#if event.url}
            <a
              class="open"
              href={event.url}
              title="باز کردن در تقویم گوگل"
              onclick={(e) => {
                e.preventDefault();
                void api.openEventLink(event.url!);
              }}
            >
              {@render row(event)}
              <span class="arrow" aria-hidden="true">↖</span>
            </a>
          {:else}
            {@render row(event)}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  </div>
  {/key}
</section>

<style>
  section {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
  }

  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
  }

  h2.holiday {
    color: var(--holiday);
  }

  .secondary {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--muted);
  }

  .sep {
    margin-inline: 6px;
    color: var(--faint);
  }

  .en {
    font-size: 11px;
    color: var(--faint);
  }

  .approx {
    margin-inline-start: 2px;
    cursor: help;
  }

  ul {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    font-size: 12.5px;
    padding-inline-start: 10px;
    position: relative;
  }

  li::before {
    content: "";
    position: absolute;
    inset-inline-start: 0;
    top: 0.55em;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent);
  }

  li.holiday::before {
    background: var(--holiday);
  }

  li.mourning::before {
    background: var(--mourning);
  }

  .events li {
    padding-inline-start: 0;
    align-items: flex-start;
    justify-content: flex-start;
    gap: 6px;
  }

  .events li::before {
    position: static;
    flex-shrink: 0;
    width: 3px;
    height: 1.4em;
    border-radius: 2px;
    background: var(--color);
  }

  /* Google events open in the browser. */
  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: -2px -4px;
    padding: 2px 4px;
    border-radius: 5px;
    color: inherit;
    text-decoration: none;
    transition: background 120ms ease;
  }

  .open:hover,
  .open:focus-visible {
    background: var(--hover);
  }

  .open .what {
    flex: 1;
  }

  .arrow {
    font-size: 11px;
    color: var(--muted);
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .open:hover .arrow,
  .open:focus-visible .arrow {
    opacity: 1;
  }

  .when {
    flex-shrink: 0;
    min-width: 38px;
    font-size: 11.5px;
    color: var(--muted);
  }

  .what {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .where {
    font-size: 11px;
    color: var(--muted);
  }

  .tag {
    flex-shrink: 0;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--holiday);
    background: var(--holiday-soft);
    border-radius: 999px;
    padding: 0 7px;
  }

  .empty {
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--faint);
  }
</style>
