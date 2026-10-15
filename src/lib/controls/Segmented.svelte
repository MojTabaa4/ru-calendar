<script lang="ts" generics="T extends string">
  import { onMount } from "svelte";

  let {
    options,
    value = $bindable(),
    label,
    onchange,
  }: {
    options: { value: T; label: string }[];
    value: T;
    label: string;
    /** When given, called instead of updating `value` directly. */
    onchange?: (value: T) => void;
  } = $props();

  let container = $state<HTMLElement | null>(null);
  let thumb = $state<{ left: number; width: number } | null>(null);
  // No slide for the first placement, only for changes after it.
  let ready = $state(false);

  function choose(next: T) {
    if (onchange) onchange(next);
    else value = next;
  }

  function measure() {
    const active = container?.querySelector<HTMLElement>('[aria-checked="true"]');
    thumb = active ? { left: active.offsetLeft, width: active.offsetWidth } : null;
  }

  $effect(() => {
    void value;
    void options.length;
    measure();
  });

  onMount(() => {
    // Widths change when the web font finishes loading.
    const resize = new ResizeObserver(measure);
    if (container) resize.observe(container);
    // After the first placement has been drawn (a timer, since hidden windows get no animation frames).
    const timer = setTimeout(() => (ready = true), 50);
    return () => {
      clearTimeout(timer);
      resize.disconnect();
    };
  });
</script>

<div class="segmented" role="radiogroup" aria-label={label} bind:this={container}>
  {#if thumb}
    <span
      class="thumb"
      class:ready
      style:transform="translateX({thumb.left}px)"
      style:width="{thumb.width}px"
      aria-hidden="true"
    ></span>
  {/if}
  {#each options as option (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === option.value}
      class:active={value === option.value}
      onclick={() => choose(option.value)}>{option.label}</button
    >
  {/each}
</div>

<style>
  .segmented {
    position: relative;
    display: inline-flex;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 2px;
  }

  /* The raised background under the chosen option; it slides to the next choice. */
  .thumb {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 0;
    border-radius: 5px;
    background: var(--surface);
    box-shadow: 0 1px 2px #0002;
  }

  .thumb.ready {
    transition:
      transform 240ms cubic-bezier(0.2, 0.8, 0.2, 1),
      width 240ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  button {
    position: relative;
    font-size: 12px;
    padding: 3px 12px;
    border-radius: 5px;
    color: var(--muted);
    transition: color 160ms ease;
  }

  button.active {
    color: var(--text);
  }
</style>
