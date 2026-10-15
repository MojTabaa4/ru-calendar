<script lang="ts">
  type YearMonth = { year: number; month: number };

  let {
    year,
    names,
    viewed,
    today,
    onPick,
  }: {
    year: number;
    names: string[];
    viewed: YearMonth | null;
    today: YearMonth | null;
    onPick: (month: number) => void;
  } = $props();
</script>

<div class="months" role="listbox" aria-label="ماه‌ها">
  {#each names as name, i (i)}
    <button
      role="option"
      aria-selected={viewed?.year === year && viewed.month === i + 1}
      class:current={viewed?.year === year && viewed.month === i + 1}
      class:today={today?.year === year && today.month === i + 1}
      style:--i={i}
      onclick={() => onPick(i + 1)}>{name}</button
    >
  {/each}
</div>

<style>
  .months {
    height: 100%;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(4, 1fr);
    gap: 6px;
    padding: 4px 2px;
  }

  button {
    border-radius: var(--radius);
    font-size: 14px;
    font-weight: 600;
    transition:
      background-color 120ms ease,
      transform 90ms ease;
    /* A quick ripple from the first month to the last. */
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
