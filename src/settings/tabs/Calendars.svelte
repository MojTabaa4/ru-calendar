<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../../lib/api";
  import type { AccessStatus } from "../../lib/bindings/AccessStatus";
  import type { CalendarInfo } from "../../lib/bindings/CalendarInfo";
  import type { Settings } from "../../lib/bindings/Settings";
  import Card from "../../lib/controls/Card.svelte";
  import SwitchRow from "../../lib/controls/SwitchRow.svelte";
  import { onAppEvent } from "../../lib/events";
  import GoogleCalendars from "./GoogleCalendars.svelte";

  let { settings = $bindable(), system }: { settings: Settings; system: boolean } = $props();

  let access = $state<AccessStatus | null>(null);
  let calendars = $state<CalendarInfo[]>([]);
  let asking = $state(false);

  // Grouped by account, in the order the app sorted them.
  const groups = $derived.by(() => {
    const bySource = new Map<string, CalendarInfo[]>();
    for (const calendar of calendars) {
      const list = bySource.get(calendar.source) ?? [];
      list.push(calendar);
      bySource.set(calendar.source, list);
    }
    return [...bySource];
  });

  async function load() {
    access = await api.calendarAccess();
    calendars = access === "authorized" ? await api.calendars() : [];
  }

  async function ask() {
    asking = true;
    try {
      access = await api.requestCalendarAccess();
      await load();
    } finally {
      asking = false;
    }
  }

  function setVisible(id: string, visible: boolean) {
    const hidden = settings.calendars.hidden.filter((h) => h !== id);
    settings.calendars.hidden = visible ? hidden : [...hidden, id];
  }

  onMount(() => {
    if (!system) return;
    void load();
    return onAppEvent("calendar-store-changed", () => void load());
  });
</script>

{#if !system}
  <!-- No system calendar on this platform: only Google. -->
{:else if access === "authorized"}
  <Card title="تقویم مک">
    <SwitchRow
      label="نمایش رویدادهای تقویم"
      hint="رویدادهای تقویم مک و زادروزها در تقویم ماهانه"
      bind:checked={settings.calendars.enabled}
    />
  </Card>
  {#each groups as [source, list] (source)}
    <Card title={source || "تقویم‌ها"}>
      {#each list as calendar (calendar.id)}
        <label class="calendar" class:off={!settings.calendars.enabled}>
          <span class="swatch" style:background={calendar.color}></span>
          <span class="name">{calendar.birthdays ? "زادروزها" : calendar.title}</span>
          <input
            type="checkbox"
            checked={!settings.calendars.hidden.includes(calendar.id)}
            disabled={!settings.calendars.enabled}
            onchange={(e) => setVisible(calendar.id, e.currentTarget.checked)}
          />
        </label>
      {/each}
    </Card>
  {:else}
    <p class="note">تقویمی پیدا نشد.</p>
  {/each}
{:else if access}
  <Card title="تقویم مک">
    <div class="access">
      {#if access === "notDetermined"}
        <p>روز می‌تواند رویدادهای تقویم مک و زادروزهای مخاطبان را در تقویم ماهانه نشان دهد. رویدادها از رایانهٔ شما خارج نمی‌شوند.</p>
        <button type="button" onclick={ask} disabled={asking}>{asking ? "در انتظار پاسخ…" : "اجازهٔ دسترسی به تقویم"}</button>
      {:else if access === "denied"}
        <p>
          دسترسی به تقویم داده نشده است. در «تنظیمات سیستم ← حریم خصوصی و امنیت ← تقویم‌ها» به روز اجازهٔ دسترسی کامل
          بدهید.
        </p>
      {:else}
        <p>دسترسی به تقویم روی این رایانه محدود شده است.</p>
      {/if}
    </div>
  </Card>
{/if}

<GoogleCalendars bind:settings />

<style>
  .calendar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    font-size: 13px;
    cursor: pointer;
  }

  .calendar.off {
    opacity: 0.5;
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .name {
    flex: 1;
  }

  input[type="checkbox"] {
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
  }

  .access {
    padding: 12px 14px;
    font-size: 13px;
  }

  .access p {
    margin: 0 0 10px;
    color: var(--muted);
  }

  .access p:last-child {
    margin-bottom: 0;
  }

  .access button {
    font-size: 13px;
    padding: 5px 14px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #fff;
  }

  .access button:disabled {
    opacity: 0.6;
  }

  .note {
    font-size: 12px;
    color: var(--muted);
  }
</style>
