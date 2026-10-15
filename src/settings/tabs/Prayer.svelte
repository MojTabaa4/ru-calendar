<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../../lib/api";
  import type { City } from "../../lib/bindings/City";
  import type { PrayerDayView } from "../../lib/bindings/PrayerDayView";
  import type { Settings } from "../../lib/bindings/Settings";
  import Card from "../../lib/controls/Card.svelte";
  import Field from "../../lib/controls/Field.svelte";
  import Segmented from "../../lib/controls/Segmented.svelte";
  import SwitchRow from "../../lib/controls/SwitchRow.svelte";
  import { onAppEvent } from "../../lib/events";

  let { settings = $bindable(), location: canLocate }: { settings: Settings; location: boolean } = $props();

  let locating = $state(false);
  let locateError = $state<string | null>(null);

  async function useMyLocation() {
    locating = true;
    locateError = null;
    try {
      const here = await api.locateMe();
      settings.prayer.location = {
        kind: "custom",
        // Within a city's limits, its name says more than "my location".
        name: here.distanceKm < 30 ? here.nearest.fa : "موقعیت من",
        latitude: Math.round(here.latitude * 10_000) / 10_000,
        longitude: Math.round(here.longitude * 10_000) / 10_000,
        timeZone: here.timeZone,
      };
    } catch (e) {
      locateError = String(e).includes("denied")
        ? "دسترسی به موقعیت داده نشد. در «تنظیمات سیستم ← حریم خصوصی و امنیت ← خدمات موقعیت‌یابی» به روز اجازه بدهید."
        : "موقعیت شما پیدا نشد.";
    } finally {
      locating = false;
    }
  }

  let query = $state("");
  let results = $state<City[]>([]);
  let today = $state<PrayerDayView | null>(null);
  let lastCity = "tehran";

  const location = $derived(settings.prayer.location);
  const mode = $derived(location.kind);

  function setMode(next: "city" | "custom") {
    if (next === location.kind) return;
    if (location.kind === "city") lastCity = location.id;
    settings.prayer.location =
      next === "city"
        ? { kind: "city", id: lastCity }
        : {
            kind: "custom",
            name: "",
            latitude: 35.6892,
            longitude: 51.389,
            timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          };
  }

  function pick(city: City) {
    settings.prayer.location = { kind: "city", id: city.id };
  }

  // Custom fields commit on change, so a half-typed number is never saved.
  function setCustom(field: "name" | "latitude" | "longitude" | "timeZone", raw: string) {
    if (location.kind !== "custom") return;
    if (field === "latitude" || field === "longitude") {
      const value = Number(raw.replace(/[۰-۹]/g, (d) => String("۰۱۲۳۴۵۶۷۸۹".indexOf(d))).replace("٫", "."));
      if (!Number.isFinite(value)) return;
      settings.prayer.location = { ...location, [field]: value };
    } else {
      settings.prayer.location = { ...location, [field]: raw.trim() };
    }
  }

  $effect(() => {
    const q = query;
    let current = true;
    api.searchCities(q).then((cities) => {
      if (current) results = cities;
    });
    return () => (current = false);
  });

  async function loadToday() {
    today = await api.prayerTimes().catch(() => null);
  }

  onMount(() => {
    void loadToday();
    return onAppEvent("settings-changed", () => void loadToday());
  });
</script>

<Card>
  <SwitchRow
    label="نمایش اوقات شرعی در تقویم"
    hint="به روش مؤسسهٔ ژئوفیزیک دانشگاه تهران"
    bind:checked={settings.prayer.enabled}
  />
</Card>

<Card title="مکان">
  <Field label="بر اساس">
    <Segmented
      label="نوع مکان"
      options={[
        { value: "city", label: "شهر" },
        { value: "custom", label: "مختصات" },
      ]}
      value={mode}
      onchange={setMode}
    />
  </Field>
  {#if canLocate}
    <div class="locate">
      <button type="button" onclick={useMyLocation} disabled={locating}>
        {locating ? "در حال یافتن موقعیت…" : "استفاده از موقعیت من"}
      </button>
      {#if locateError}<p>{locateError}</p>{/if}
    </div>
  {/if}
  {#if location.kind === "city"}
    <div class="search">
      <input type="search" placeholder="جست‌وجوی شهر…" aria-label="جست‌وجوی شهر" bind:value={query} />
      <ul role="listbox" aria-label="شهرها">
        {#each results as city (city.id)}
          <li role="option" aria-selected={city.id === location.id}>
            <button type="button" class:selected={city.id === location.id} onclick={() => pick(city)}>
              <span>{city.fa}</span>
              <span class="en" dir="ltr">{city.en}</span>
            </button>
          </li>
        {:else}
          <li class="empty">شهری پیدا نشد.</li>
        {/each}
      </ul>
    </div>
  {:else}
    <Field label="نام">
      <input
        class="text"
        value={location.name}
        placeholder="مکان من"
        onchange={(e) => setCustom("name", e.currentTarget.value)}
      />
    </Field>
    <Field label="عرض جغرافیایی" hint="شمال مثبت">
      <input
        class="number"
        dir="ltr"
        inputmode="decimal"
        value={location.latitude}
        onchange={(e) => setCustom("latitude", e.currentTarget.value)}
      />
    </Field>
    <Field label="طول جغرافیایی" hint="شرق مثبت">
      <input
        class="number"
        dir="ltr"
        inputmode="decimal"
        value={location.longitude}
        onchange={(e) => setCustom("longitude", e.currentTarget.value)}
      />
    </Field>
    <Field label="منطقهٔ زمانی" hint="مثلاً Asia/Tehran">
      <input
        class="text"
        dir="ltr"
        value={location.timeZone}
        onchange={(e) => setCustom("timeZone", e.currentTarget.value)}
      />
    </Field>
  {/if}
</Card>

{#if today}
  <Card title="امروز به افق {today.place}">
    <ol class="today">
      {#each today.times as time (time.prayer)}
        <li class:next={time.next}><span>{time.name}</span><span>{time.time}</span></li>
      {/each}
    </ol>
  </Card>
{/if}

<style>
  .search {
    padding: 10px 14px;
  }

  .locate {
    padding: 10px 14px;
  }

  .locate button {
    font-size: 12.5px;
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .locate button:hover:not(:disabled) {
    background: var(--hover);
  }

  .locate p {
    margin: 6px 0 0;
    font-size: 11.5px;
    color: var(--holiday);
  }

  input {
    font: inherit;
    font-size: 13px;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    height: 30px;
    padding: 0 10px;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .search input {
    width: 100%;
  }

  .text {
    width: 180px;
  }

  .number {
    width: 110px;
    text-align: center;
  }

  ul {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    max-height: 168px;
    overflow-y: auto;
  }

  ul button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    font-size: 13px;
  }

  ul button:hover {
    background: var(--hover);
  }

  ul button.selected {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .en {
    font-size: 11.5px;
    color: var(--muted);
  }

  .empty {
    padding: 6px 8px;
    font-size: 12px;
    color: var(--faint);
  }

  .today {
    list-style: none;
    margin: 0;
    padding: 6px 14px;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px 16px;
    font-size: 12.5px;
  }

  .today li {
    display: flex;
    justify-content: space-between;
  }

  .today li.next {
    color: var(--accent);
    font-weight: 600;
  }
</style>
