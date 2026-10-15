<script lang="ts">
  import type { Settings } from "../../lib/bindings/Settings";
  import type { Weekday } from "../../lib/bindings/Weekday";
  import Card from "../../lib/controls/Card.svelte";
  import Field from "../../lib/controls/Field.svelte";
  import Segmented from "../../lib/controls/Segmented.svelte";
  import SwitchRow from "../../lib/controls/SwitchRow.svelte";

  let { settings = $bindable() }: { settings: Settings } = $props();

  const weekdays: { value: Weekday; short: string; name: string }[] = [
    { value: "saturday", short: "ش", name: "شنبه" },
    { value: "sunday", short: "ی", name: "یکشنبه" },
    { value: "monday", short: "د", name: "دوشنبه" },
    { value: "tuesday", short: "س", name: "سه‌شنبه" },
    { value: "wednesday", short: "چ", name: "چهارشنبه" },
    { value: "thursday", short: "پ", name: "پنجشنبه" },
    { value: "friday", short: "ج", name: "جمعه" },
  ];

  function toggleWeekend(day: Weekday) {
    settings.weekend = settings.weekend.includes(day)
      ? settings.weekend.filter((d) => d !== day)
      : [...settings.weekend, day];
  }
</script>

<Card>
  <SwitchRow
    label="اجرا هنگام ورود به سیستم"
    hint="روز پس از روشن شدن رایانه خودکار باز می‌شود."
    bind:checked={settings.startAtLogin}
  />
</Card>

<Card title="تقویم">
  <Field label="نام ماه‌ها و روزها">
    <Segmented
      label="زبان نام ماه‌ها و روزها"
      options={[
        { value: "fa", label: "فارسی" },
        { value: "en", label: "English" },
      ]}
      bind:value={settings.locale}
    />
  </Field>
  <Field label="ارقام">
    <Segmented
      label="ارقام"
      options={[
        { value: "persian", label: "۱۲۳" },
        { value: "latin", label: "123" },
      ]}
      bind:value={settings.digits}
    />
  </Field>
  <Field label="تعطیلی هفته" hint="این روزها در تقویم قرمز نشان داده می‌شوند.">
    <div class="weekdays" role="group" aria-label="روزهای تعطیل هفته">
      {#each weekdays as day (day.value)}
        <button
          type="button"
          title={day.name}
          aria-pressed={settings.weekend.includes(day.value)}
          class:on={settings.weekend.includes(day.value)}
          onclick={() => toggleWeekend(day.value)}>{day.short}</button
        >
      {/each}
    </div>
  </Field>
  <Field label="امروز بر اساس" hint="تعیین می‌کند «امروز» کدام روز است.">
    <Segmented
      label="منطقهٔ زمانی"
      options={[
        { value: "system", label: "ساعت رایانه" },
        { value: "tehran", label: "ساعت تهران" },
      ]}
      bind:value={settings.timeZone}
    />
  </Field>
  <SwitchRow label="نمایش روز میلادی در تقویم" bind:checked={settings.showGregorian} />
  <SwitchRow label="نمایش روز قمری در تقویم" bind:checked={settings.showHijri} />
</Card>

<style>
  .weekdays {
    display: flex;
    gap: 3px;
  }

  .weekdays button {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 600;
    background: var(--surface-2);
    color: var(--muted);
  }

  .weekdays button.on {
    background: var(--holiday);
    color: #fff;
  }
</style>
