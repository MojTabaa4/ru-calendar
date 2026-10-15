<script lang="ts">
  import { onMount } from "svelte";
  import { slide } from "svelte/transition";
  import { api } from "../../lib/api";
  import type { GoogleStatus } from "../../lib/bindings/GoogleStatus";
  import type { Settings } from "../../lib/bindings/Settings";
  import Card from "../../lib/controls/Card.svelte";
  import SwitchRow from "../../lib/controls/SwitchRow.svelte";
  import { reducedMotion } from "../../lib/motion";

  let { settings = $bindable() }: { settings: Settings } = $props();

  let status = $state<GoogleStatus | null>(null);
  let connecting = $state(false);
  let error = $state<string | null>(null);
  // "Disconnect" asks for a second click before signing out.
  let confirming = $state<string | null>(null);
  let disconnecting = $state<string | null>(null);

  const duration = reducedMotion() ? 0 : 200;

  async function load() {
    try {
      status = await api.googleStatus();
    } catch (e) {
      error = String(e);
    }
  }

  async function connect() {
    connecting = true;
    error = null;
    try {
      if (await api.googleConnect()) await load();
    } catch (e) {
      error = String(e);
    } finally {
      connecting = false;
    }
  }

  async function disconnect(email: string) {
    if (confirming !== email) {
      confirming = email;
      setTimeout(() => {
        if (confirming === email) confirming = null;
      }, 4000);
      return;
    }
    confirming = null;
    disconnecting = email;
    try {
      await api.googleDisconnect(email);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      disconnecting = null;
    }
  }

  function hidden(email: string): string[] {
    return settings.google.accounts.find((a) => a.email === email)?.hidden ?? [];
  }

  function setVisible(email: string, id: string, visible: boolean) {
    const account = settings.google.accounts.find((a) => a.email === email);
    if (!account) return;
    const rest = account.hidden.filter((h) => h !== id);
    account.hidden = visible ? rest : [...rest, id];
  }

  onMount(() => {
    void load();
    // Coming back to the window, e.g. after adding google-client.json or signing in.
    const refresh = () => void load();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  });
</script>

<Card title="تقویم گوگل">
  {#if !status}
    <p class="pad note">در حال بارگذاری…</p>
  {:else if !status.configured}
    <div class="pad">
      <p class="note">
        برای اتصال به تقویم گوگل، روز به یک «کلاینت OAuth» از Google Cloud نیاز دارد. فایل JSON آن را با نام
        <code dir="ltr">google-client.json</code> در پوشهٔ تنظیمات روز بگذارید (راهنما در README).
      </p>
      <button type="button" class="primary" onclick={() => void api.openConfigFolder()}>باز کردن پوشهٔ تنظیمات</button>
    </div>
  {:else}
    {#if status.accounts.length > 0}
      <SwitchRow
        label="نمایش رویدادهای گوگل"
        hint="رویدادهای تقویم‌های انتخاب‌شده در تقویم ماهانه"
        bind:checked={settings.google.enabled}
      />
    {/if}
    <div class="pad connect">
      {#if connecting}
        <span class="waiting"><span class="spinner" aria-hidden="true"></span>ورود را در مرورگر کامل کنید…</span>
        <button type="button" class="plain" onclick={() => void api.googleCancelSignIn()}>لغو</button>
      {:else}
        <span class="note">
          {status.accounts.length > 0
            ? "می‌توانید چند حساب گوگل را با هم وصل کنید."
            : "روز فقط رویدادها را می‌خواند و چیزی را در تقویم شما تغییر نمی‌دهد."}
        </span>
        <button type="button" class="primary" onclick={connect}>
          {status.accounts.length > 0 ? "افزودن حساب" : "اتصال به گوگل"}
        </button>
      {/if}
    </div>
  {/if}
  {#if error}<p class="pad error" role="alert" transition:slide={{ duration }}>{error}</p>{/if}
</Card>

{#each status?.accounts ?? [] as account (account.email)}
  <div transition:slide={{ duration }}>
    <Card>
      <div class="account">
        <span class="avatar" aria-hidden="true">{account.email[0]?.toUpperCase()}</span>
        <span class="email" dir="ltr">{account.email}</span>
        <button
          type="button"
          class="plain"
          class:danger={confirming === account.email}
          disabled={disconnecting === account.email}
          onclick={() => disconnect(account.email)}
        >
          {disconnecting === account.email ? "…" : confirming === account.email ? "مطمئنید؟" : "قطع اتصال"}
        </button>
      </div>
      {#if account.error}
        <div class="pad problem">
          <span>{account.error}</span>
          <button type="button" class="plain" onclick={connect} disabled={connecting}>اتصال دوباره</button>
        </div>
      {/if}
      {#each account.calendars as calendar (calendar.id)}
        <label class="calendar" class:off={!settings.google.enabled}>
          <span class="swatch" style:background={calendar.color}></span>
          <span class="name">{calendar.title}</span>
          <input
            type="checkbox"
            checked={!hidden(account.email).includes(calendar.id)}
            disabled={!settings.google.enabled}
            onchange={(e) => setVisible(account.email, calendar.id, e.currentTarget.checked)}
          />
        </label>
      {/each}
    </Card>
  </div>
{/each}

<style>
  .pad {
    margin: 0;
    padding: 10px 14px;
    font-size: 13px;
  }

  .note {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  div.pad .note {
    margin-bottom: 10px;
  }

  code {
    font-size: 11.5px;
    padding: 0 4px;
    border-radius: 4px;
    background: var(--hover);
  }

  .connect {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .connect .note {
    margin: 0;
  }

  .waiting {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
  }

  .spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--accent-soft);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  button {
    flex-shrink: 0;
    font-size: 12.5px;
    padding: 4px 12px;
    border-radius: var(--radius-sm);
    transition:
      background 120ms ease,
      color 120ms ease,
      opacity 120ms ease;
  }

  .primary {
    background: var(--accent);
    color: #fff;
  }

  .plain {
    color: var(--accent);
  }

  .plain:hover:not(:disabled) {
    background: var(--accent-soft);
  }

  .plain.danger,
  .plain.danger:hover:not(:disabled) {
    color: #fff;
    background: var(--holiday);
  }

  button:disabled {
    opacity: 0.5;
  }

  .error {
    color: var(--holiday);
    background: var(--holiday-soft);
  }

  .account {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
  }

  .avatar {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 700;
    color: #fff;
    background: #4285f4;
  }

  .email {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
    font-size: 13px;
    font-weight: 600;
  }

  .problem {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: var(--holiday);
    font-size: 12px;
  }

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
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  input[type="checkbox"] {
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
  }
</style>
