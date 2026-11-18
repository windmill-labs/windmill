<script lang="ts">
	import { navHandleSlot } from './navHandlePlacement.svelte'
</script>

<!-- Placed in a `relative` box around the sidebar mode button. `opacity-0` at rest: the keyframes
     carry no fill mode, so the element returns to its own opacity once the pulse ends. -->
{#if navHandleSlot.modeFlash}
	{#key navHandleSlot.modeFlash}
		<span
			aria-hidden="true"
			class="pointer-events-none absolute -inset-1 rounded-lg bg-blue-500/30 opacity-0 motion-safe:animate-[fade-out_1.2s_ease-in-out_0.2s]"
		></span>
	{/key}
{/if}
