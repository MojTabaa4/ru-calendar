<script lang="ts">
  import { api } from "../lib/api";

  let { selected }: { selected: number | null } = $props();
</script>

<footer>
  <span class="name">روز</span>
  <div class="actions">
    <button
      onclick={() => api.openWindow("converter", selected ?? undefined)}
      title="تبدیل تاریخ"
      aria-label="تبدیل تاریخ"
    >
      <svg viewBox="0 0 20 20" aria-hidden="true">
        <path d="M4 7h11l-3-3M16 13H5l3 3" />
      </svg>
    </button>
    <button onclick={() => api.openWindow("settings")} title="تنظیمات" aria-label="تنظیمات">
      <svg viewBox="0 0 20 20" aria-hidden="true">
        <circle cx="10" cy="10" r="2.6" />
        <path
          d="M10 2.5l1.3 1.9 2.2-.6.5 2.2 2.2.6-.6 2.2 1.9 1.2-1.9 1.3.6 2.2-2.2.5-.5 2.2-2.2-.6L10 17.5l-1.3-1.9-2.2.6-.5-2.2-2.2-.5.6-2.2-1.9-1.3 1.9-1.2-.6-2.2 2.2-.6.5-2.2 2.2.6z"
        />
      </svg>
    </button>
    <button onclick={() => api.quit()} title="خروج از روز">خروج</button>
  </div>
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-inline: 4px;
    font-size: 12px;
    color: var(--muted);
  }

  .name {
    font-weight: 700;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  button {
    display: grid;
    place-items: center;
    min-width: 26px;
    height: 24px;
    padding: 0 8px;
    border-radius: var(--radius-sm);
  }

  button:hover {
    background: var(--hover);
    color: var(--text);
  }

  svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
</style>
