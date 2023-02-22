<script lang="ts">
	import type { ComponentCssProperty } from "../../types"

	export let name: string
	export let value: ComponentCssProperty | undefined
</script>

<div class="text-sm font-semibold text-gray-500 capitalize pt-2">
	{name}
</div>
{#if value}
	<div class="border-l border-gray-400/80 py-1 pl-3.5 mt-1 ml-0.5">
		<label class="block pb-2">
			<div class="text-xs font-medium pb-0.5">
				Style
			</div>
			<input
				type="text"
				bind:value={value.style}
				on:focus
			/>
		</label>
		<label class="block">
			<div class="text-xs font-medium pb-0.5">
				Tailwind classes
			</div>
			<input
				type="text"
				bind:value={value.class}
				on:focus
			/>
		</label>
	</div>
{/if}