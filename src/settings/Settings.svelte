<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import type { Features } from "../lib/bindings/Features";
  import type { Settings } from "../lib/bindings/Settings";
  import Segmented from "../lib/controls/Segmented.svelte";
  import { onAppEvent } from "../lib/events";
  import { timeline } from "../lib/motion";
  import About from "./tabs/About.svelte";
  import Calendars from "./tabs/Calendars.svelte";
  import General from "./tabs/General.svelte";
  import MenuBar from "./tabs/MenuBar.svelte";
  import Occasions from "./tabs/Occasions.svelte";
  import Prayer from "./tabs/Prayer.svelte";

  type Tab = "general" | "menuBar" | "occasions" | "calendars" | "prayer" | "about";

  const isMac = navigator.userAgent.includes("Mac");
  const allTabs: { value: Tab; label: string }[] = [
    { value: "general", label: "عمومی" },
    { value: "menuBar", label: isMac ? "نوار منو" : "سینی سیستم" },
    { value: "occasions", label: "مناسبت‌ها" },
    { value: "calendars", label: "تقویم‌ها" },
    { value: "prayer", label: "اوقات شرعی" },
    { value: "about", label: "درباره" },
  ];
  let features = $state<Features>({ calendars: false, location: false });

  let tab = $state<Tab>("general");
  // Tab contents slide toward the side of the tab chosen: 1 for one further along.
  // Set together with the tab, so the transition starting now sees it.
  let tabDirection = $state(0);
  function chooseTab(next: Tab) {
    const order = allTabs.map((t) => t.value);
    tabDirection = Math.sign(order.indexOf(next) - order.indexOf(tab));
    tab = next;
  }
  let settings = $state<Settings | null>(null);
  let error = $state<string | null>(null);
  // JSON of what the app last confirmed, so only real edits are sent.
  let confirmed = "";

  async function load() {
    const current = await api.settings();
    confirmed = JSON.stringify(current);
    settings = current;
  }

  // Changes apply as soon as they're made; a short delay batches quick clicks.
  $effect(() => {
    if (!settings) return;
    const json = JSON.stringify(settings);
    if (json === confirmed) return;
    const snapshot = $state.snapshot(settings) as Settings;
    const timer = setTimeout(async () => {
      try {
        const saved = await api.updateSettings(snapshot);
        error = null;
        confirmed = JSON.stringify(saved);
        settings = saved;
      } catch (e) {
        error = String(e);
        await load();
      }
    }, 150);
    return () => clearTimeout(timer);
  });

  onMount(() => {
    load().catch((e) => (error = String(e)));
    api.features().then((f) => (features = f)).catch(() => {});
    // Settings also change outside this window, e.g. signing in to Google adds an account.
    return onAppEvent<Settings>("settings-changed", (next) => {
      const json = JSON.stringify(next);
      if (json === confirmed) return;
      confirmed = json;
      settings = next;
    });
  });
</script>

<main>
  <nav>
    <Segmented options={allTabs} value={tab} onchange={chooseTab} label="بخش‌های تنظیمات" />
  </nav>
  <div class="content stack">
    {#if settings}
      {#key tab}
      <div in:timeline={{ direction: tabDirection, distance: 16, duration: 240 }} out:timeline={{ direction: tabDirection, distance: 16, duration: 160 }}>
      {#if tab === "general"}
        <General bind:settings />
      {:else if tab === "menuBar"}
        <MenuBar bind:settings {isMac} />
      {:else if tab === "occasions"}
        <Occasions bind:settings />
      {:else if tab === "calendars"}
        <Calendars bind:settings system={features.calendars} />
      {:else if tab === "prayer"}
        <Prayer bind:settings location={features.location} />
      {:else}
        <About />
      {/if}
      </div>
      {/key}
    {/if}
  </div>
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</main>

<style>
  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  nav {
    display: flex;
    justify-content: center;
    padding: 14px 16px 10px;
    border-bottom: 1px solid var(--border);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px 8px;
  }

  .error {
    margin: 0;
    padding: 8px 16px;
    font-size: 12px;
    color: var(--holiday);
    background: var(--holiday-soft);
  }
</style>
