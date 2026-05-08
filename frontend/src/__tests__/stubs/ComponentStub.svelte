<script lang="ts">
  export let name = "stub";
  export let surface = false;
</script>

<div data-testid={`stub-${name}`} data-surface={surface ? "true" : "false"}></div>
