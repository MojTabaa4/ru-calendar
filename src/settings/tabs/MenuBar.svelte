<script lang="ts">
  import { api } from "../../lib/api";
  import type { Settings } from "../../lib/bindings/Settings";
  import Card from "../../lib/controls/Card.svelte";
  import Field from "../../lib/controls/Field.svelte";
  import Segmented from "../../lib/controls/Segmented.svelte";
  import SwitchRow from "../../lib/controls/SwitchRow.svelte";

  let { settings = $bindable(), isMac }: { settings: Settings; isMac: boolean } = $props();

  let preview = $state("");

  $effect(() => {
    const tray = $state.snapshot(settings.tray);
    let current = true;
    api.trayPreview(tray).then((text) => {
      if (current) preview = text;
    });
    return () => (current = false);
  });
</script>

<div class="preview" aria-label="پیش‌نمایش">
  <!-- The menu bar itself runs left to right, with status items at its right end. -->
  <div class="bar" dir="ltr">
    <span class="title" dir="rtl">{preview}</span>
  </div>
  {#if !isMac}
    <p>
      در ویندوز و لینوکس، شمارهٔ روز روی آیکن دیده می‌شود و این متن در راهنمای آیکن (و در برخی
      میزکارهای لینوکس کنار آن).
    </p>
  {/if}
</div>

<Card title="زبان">
  <Field label="نام روز و ماه" hint="«پنجشنبه ۲۳ مهر» یا «Thursday 23 Mehr»">
    <Segmented
      label="زبان نام روز و ماه در نوار منو"
      options={[
        { value: "fa", label: "مهر" },
        { value: "en", label: "Mehr" },
      ]}
      value={settings.tray.format.locale}
      onchange={(locale) => {
        // Digits follow the language; they can still be changed below.
        settings.tray.format.locale = locale;
        settings.tray.format.digits = locale === "en" ? "latin" : "persian";
      }}
    />
  </Field>
  <Field label="ارقام">
    <Segmented
      label="ارقام"
      options={[
        { value: "persian", label: "۱۲۳" },
        { value: "latin", label: "123" },
      ]}
      bind:value={settings.tray.format.digits}
    />
  </Field>
</Card>

<Card title="چه نمایش داده شود">
  <SwitchRow label="روز هفته" bind:checked={settings.tray.format.showWeekday} />
  <SwitchRow label="نام ماه" bind:checked={settings.tray.format.showMonth} />
  <SwitchRow label="سال" bind:checked={settings.tray.format.showYear} />
  <SwitchRow label="ساعت" hint="به وقت منطقهٔ زمانی انتخاب‌شده در «عمومی»" bind:checked={settings.tray.showTime} />
</Card>

<style>
  .preview {
    margin-bottom: 16px;
  }

  .bar {
    display: flex;
    justify-content: flex-end;
    padding: 5px 14px;
    border-radius: var(--radius-sm);
    background: linear-gradient(var(--surface-2), var(--surface));
    border: 1px solid var(--border);
    min-height: 30px;
  }

  .title {
    font-size: 13px;
    font-weight: 500;
  }

  p {
    margin: 6px 4px 0;
    font-size: 11.5px;
    color: var(--muted);
  }
</style>
