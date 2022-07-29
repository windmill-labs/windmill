<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import IconedResourceType from './IconedResourceType.svelte'

	let resources: string[] = []

	export let value: string | undefined

	export let notPickable = false
	export let nonePickable = false

	async function loadResources() {
		resources = await ResourceService.listResourceTypeNames({ workspace: $workspaceStore! })
	}

	const dispatch = createEventDispatcher()

	$: {
		if ($workspaceStore) {
			loadResources()
		}
	}
</script>

<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
	{#if nonePickable}
		<button
			class="px-4 h-8 text-center {undefined == value
				? 'item-button-selected'
				: notPickable
				? 'item-button-disabled'
				: 'item-button'}"
			on:click={() => {
				value = undefined
				dispatch('click')
			}}
		>
			None
		</button>
	{/if}
	{#each resources as r}
		<button
			class="px-4 h-8 text-center {r == value
				? 'item-button-selected'
				: notPickable
				? 'item-button-disabled'
				: 'item-button'}"
			on:click={() => {
				value = r
				dispatch('click')
			}}
		>
			<IconedResourceType name={r} after={true} />
		</button>
	{/each}
</div>

<style>
	.selected:hover {
		@apply border border-gray-400 rounded-md border-opacity-50;
	}
</style>
