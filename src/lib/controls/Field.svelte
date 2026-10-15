<script lang="ts">
  import type { Snippet } from "svelte";

  let { label, hint, children }: { label: string; hint?: string; children: Snippet } = $props();
</script>

<div class="row">
  <span class="text">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <span class="control">{@render children()}</span>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 14px;
  }

  .text {
    display: flex;
    flex-direction: column;
  }

  .label {
    font-size: 13px;
  }

  .hint {
    font-size: 11.5px;
    color: var(--muted);
  }

  .control {
    flex-shrink: 0;
  }
</style>
