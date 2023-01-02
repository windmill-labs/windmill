<script lang="ts">
	import { APP_TO_ICON_COMPONENT } from './icons'

	export let name: string
	export let silent = false
	export let after: boolean = false
	export let height = '24px'
	export let width = '24px'
	export let center = false

	$: name_prefix = name.split('_')[0]
</script>

<div class="truncate flex flex-row gap-2 {center ? 'justify-center items-center' : ''}  -pl-2">
	{#if !silent && !after}
		{name}
	{/if}
	{#if APP_TO_ICON_COMPONENT[name_prefix]}
		<span class="text-gray-700 grayscale">
			<svelte:component this={APP_TO_ICON_COMPONENT[name_prefix]} {height} {width} />
		</span>
	{:else}
		<span style="width: {width}" class="bg-gray-100 rounded-full" />
	{/if}
	{#if !silent && after}
		{name}
	{/if}
</div>
