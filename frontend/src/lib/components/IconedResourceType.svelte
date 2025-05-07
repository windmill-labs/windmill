<script>
	import { FileText } from 'lucide-svelte'
	import { APP_TO_ICON_COMPONENT } from './icons'
	export let name
	export let silent = false
	export let after = false
	export let height = '24px'
	export let width = '24px'
	export let center = false
	export let isSelected = false
	export let formatExtension = undefined

	$: iconComponent = name === "teams" 
		? APP_TO_ICON_COMPONENT.ms_teams_webhook 
		: APP_TO_ICON_COMPONENT[name] || APP_TO_ICON_COMPONENT[name.split('_')[0]]
</script>

<div class="truncate flex flex-row gap-2 {center ? 'justify-center items-center' : ''}  -pl-2">
	{#if !silent && !after}
		{name}
	{/if}
	{#if iconComponent}
		<span class={isSelected ? 'text-secondary' : 'text-secondary grayscale'}>
			<svelte:component this={iconComponent} {height} {width} />
		</span>
	{:else if formatExtension}
		<span class={isSelected ? 'text-secondary' : 'text-secondary grayscale'}>
			<svelte:component this={FileText} {height} {width} />
		</span>
	{:else}
		<span style="width: {width}; height: {height}" class="bg-gray-100 rounded-full"></span>
	{/if}
	{#if !silent && after}
		{name}
	{/if}
</div>

