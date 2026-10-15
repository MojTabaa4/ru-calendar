<script lang="ts">
  let {
    label,
    hint,
    checked = $bindable(),
    disabled = false,
  }: { label: string; hint?: string; checked: boolean; disabled?: boolean } = $props();
</script>

<label class="row" class:disabled>
  <span class="text">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" role="switch" bind:checked {disabled} />
</label>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 14px;
    cursor: pointer;
  }

  .disabled {
    opacity: 0.5;
    cursor: default;
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

  input {
    appearance: none;
    flex-shrink: 0;
    width: 34px;
    height: 20px;
    border-radius: 999px;
    background: var(--faint);
    position: relative;
    cursor: inherit;
    transition: background 0.15s;
    margin: 0;
  }

  input::after {
    content: "";
    position: absolute;
    top: 2px;
    inset-inline-start: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px #0003;
    transition: inset-inline-start 0.15s;
  }

  input:checked {
    background: var(--accent);
  }

  input:checked::after {
    inset-inline-start: 16px;
  }

  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
