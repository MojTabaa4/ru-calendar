<script lang="ts">
  import type { Settings } from "../../lib/bindings/Settings";
  import Card from "../../lib/controls/Card.svelte";
  import Field from "../../lib/controls/Field.svelte";
  import SwitchRow from "../../lib/controls/SwitchRow.svelte";

  let { settings = $bindable() }: { settings: Settings } = $props();

  const digits = "۰۱۲۳۴۵۶۷۸۹";
  const signed = (n: number) =>
    (n > 0 ? "+" : n < 0 ? "−" : "") + String(Math.abs(n)).replace(/\d/g, (d) => digits[Number(d)]);

  function shift(delta: number) {
    settings.hijri.offset = Math.max(-2, Math.min(2, settings.hijri.offset + delta));
  }
</script>

<Card title="نمایش مناسبت‌ها">
  <SwitchRow
    label="تعطیلات و مناسبت‌های رسمی"
    hint="تقویم رسمی ایران"
    bind:checked={settings.occasions.officialMain}
  />
  <SwitchRow
    label="روزهای ملی و بزرگداشت‌ها"
    hint="بزرگداشت شاعران و دانشمندان، روزهای ملی و بین‌المللی"
    bind:checked={settings.occasions.officialAppendix}
  />
  <SwitchRow label="روز مشاغل" hint="روز پزشک، روز مهندس و…" bind:checked={settings.occasions.officialProfessions} />
  <SwitchRow
    label="جشن‌های ایرانی"
    hint="مهرگان، یلدا، سده، چهارشنبه‌سوری و…"
    bind:checked={settings.occasions.historical}
  />
  <SwitchRow label="مناسبت‌های دیگر شیعی" bind:checked={settings.occasions.shiaExtra} />
</Card>

<Card title="تاریخ قمری">
  <SwitchRow
    label="تاریخ‌های رسمی ایران"
    hint="ماه‌های قمری طبق تقویم رسمی ایران؛ در غیر این صورت طبق تقویم ام‌القری"
    bind:checked={settings.hijri.iranAdjustment}
  />
  <Field label="جابه‌جایی" hint="اگر آغاز ماه با رؤیت هلال تغییر کرد">
    <div class="stepper">
      <button type="button" aria-label="یک روز عقب" onclick={() => shift(-1)} disabled={settings.hijri.offset <= -2}
        >−</button
      >
      <output aria-live="polite">{signed(settings.hijri.offset)} روز</output>
      <button type="button" aria-label="یک روز جلو" onclick={() => shift(1)} disabled={settings.hijri.offset >= 2}
        >+</button
      >
    </div>
  </Field>
</Card>

<style>
  .stepper {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .stepper button {
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-size: 15px;
    line-height: 1;
  }

  .stepper button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  output {
    min-width: 52px;
    text-align: center;
    font-size: 13px;
  }
</style>
