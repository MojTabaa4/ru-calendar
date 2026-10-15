<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { isTauri } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let version = $state("");

  onMount(() => {
    if (isTauri()) {
      getVersion().then((v) => (version = v));
    }
  });
</script>

<div class="about">
  <svg viewBox="0 0 1024 1024" class="icon" aria-hidden="true">
    <rect x="100" y="100" width="824" height="824" rx="185" fill="#f6f6f4" />
    <path d="M100 285a185 185 0 0 1 185-185h454a185 185 0 0 1 185 185v47H100z" fill="#d1312a" />
    <g fill="#d3d3d7">
      <rect x="223" y="410" width="110" height="110" rx="26" />
      <rect x="379" y="410" width="110" height="110" rx="26" />
      <rect x="535" y="410" width="110" height="110" rx="26" />
      <rect x="691" y="410" width="110" height="110" rx="26" />
      <rect x="223" y="560" width="110" height="110" rx="26" />
      <rect x="379" y="560" width="110" height="110" rx="26" />
      <rect x="691" y="560" width="110" height="110" rx="26" />
      <rect x="223" y="710" width="110" height="110" rx="26" />
      <rect x="379" y="710" width="110" height="110" rx="26" />
      <rect x="535" y="710" width="110" height="110" rx="26" />
    </g>
    <rect x="535" y="560" width="110" height="110" rx="26" fill="#d1312a" />
  </svg>
  <h1>روز</h1>
  {#if version}<p class="version">نسخهٔ <span dir="ltr">{version}</span></p>{/if}
  <p>تقویم خورشیدی، میلادی و قمری برای نوار منو و سینی سیستم.</p>
</div>

<dl>
  <dt>تعطیلات و مناسبت‌ها</dt>
  <dd>از تقویم رسمی ایران (مرکز تقویم مؤسسهٔ ژئوفیزیک دانشگاه تهران)</dd>
  <dt>ماه‌های قمری</dt>
  <dd>طبق تقویم رسمی ایران تا پایان ۱۴۰۵؛ پس از آن، تقویم ام‌القری (تقریبی)</dd>
  <dt>قلم</dt>
  <dd>وزیرمتن، اثر صابر راستی‌کردار، با پروانهٔ SIL Open Font License 1.1</dd>
</dl>

<style>
  .about {
    text-align: center;
    margin: 8px 0 20px;
  }

  .icon {
    width: 72px;
    height: 72px;
  }

  h1 {
    margin: 4px 0 0;
    font-size: 20px;
  }

  p {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--muted);
  }

  .version {
    font-size: 11.5px;
  }

  dl {
    margin: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 6px 14px;
    font-size: 12.5px;
  }

  dt {
    font-weight: 600;
    margin-top: 6px;
  }

  dd {
    margin: 0 0 6px;
    color: var(--muted);
  }
</style>
