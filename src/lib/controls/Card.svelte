<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, children }: { title?: string; children: Snippet } = $props();
</script>

<section>
  {#if title}<h3>{title}</h3>{/if}
  <div class="card">{@render children()}</div>
</section>

<style>
  section {
    margin-bottom: 14px;
  }

  h3 {
    margin: 0 4px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .card > :global(* + *) {
    border-top: 1px solid var(--border);
  }
</style>
