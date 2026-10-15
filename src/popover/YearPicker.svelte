<script lang="ts">
  import type { Digits } from "../lib/bindings/Digits";
  import { localDigits } from "../lib/digits";

  let {
    start,
    viewed,
    today,
    digits,
    onPick,
  }: {
    /** First of the twelve years shown. */
    start: number;
    viewed: number | null;
    today: number | null;
    digits: Digits;
    onPick: (year: number) => void;
  } = $props();

  const years = $derived(Array.from({ length: 12 }, (_, i) => start + i));
</script>

<div class="years" role="listbox" aria-label="سال‌ها">
  {#each years as year, i (year)}
    <button
      role="option"
      aria-selected={viewed === year}
      class:current={viewed === year}
      class:today={today === year}
      style:--i={i}
      onclick={() => onPick(year)}>{localDigits(year, digits)}</button
    >
  {/each}
</div>

<style>
  .years {
    height: 100%;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(4, 1fr);
    gap: 6px;
    padding: 4px 2px;
  }

  button {
    border-radius: var(--radius);
    font-size: 15px;
    font-weight: 600;
    transition:
      background-color 120ms ease,
      transform 90ms ease;
    animation: settle 260ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
    animation-delay: calc(var(--i) * 14ms);
  }

  @keyframes settle {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }

  button:hover {
    background: var(--hover);
  }

  button:active {
    transform: scale(0.95);
  }

  .today {
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }

  .current,
  .current:hover {
    background: var(--accent);
    color: #fff;
  }
</style>
