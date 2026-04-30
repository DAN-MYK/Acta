<script lang="ts">
  import { appIcons, type AppIconName } from "../icons";

  export let name: AppIconName;
  export let size = 20;
  export let title = "";
  export let surface = false;

  $: markup = appIcons[name];
</script>

<span
  class:surface
  class="app-icon-wrap"
  style={`--icon-size: ${size}px;`}
  aria-hidden={title ? undefined : "true"}
  role={title ? "img" : undefined}
  aria-label={title || undefined}
>
  {@html markup}
</span>

<style>
  .app-icon-wrap {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: var(--icon-size);
    height: var(--icon-size);
    color: currentColor;
  }

  .app-icon-wrap.surface {
    width: calc(var(--icon-size) + 12px);
    height: calc(var(--icon-size) + 12px);
    border-radius: 12px;
    background: color-mix(in srgb, currentColor 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, currentColor 12%, transparent);
  }

  .app-icon-wrap :global(svg) {
    display: block;
    width: var(--icon-size);
    height: var(--icon-size);
  }
</style>
