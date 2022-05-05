<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	import { ResourceService } from '../../gen';
	import { workspaceStore } from '../../stores';
	import IconedResourceType from './IconedResourceType.svelte';

	let resources: string[] = [];

	export let value: string | undefined;

	export let notPickable = false;

	async function loadResources() {
		resources = await ResourceService.listResourceTypeNames({ workspace: $workspaceStore! });
	}

	const dispatch = createEventDispatcher();

	$: {
		if ($workspaceStore) {
			loadResources();
		}
	}
</script>

<div class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center mb-2">
	{#each resources as r}
		<button
			class="px-4 h-8 {r == value
				? 'item-button-selected'
				: notPickable
				? 'item-button-disabled'
				: 'item-button'}"
			on:click={() => {
				value = r;
				dispatch('click');
			}}
		>
			<IconedResourceType name={r} after={true} />
		</button>
	{/each}
</div>

<style>
	.item-button {
		@apply py-1;
		@apply border;
		@apply rounded-sm;
	}
	.item-button-selected {
		@apply py-1;
		@apply border border-blue-500;
		@apply bg-blue-50;
		@apply rounded-sm;
	}

	.item-button-disabled {
		@apply py-1;
		@apply border;
		@apply bg-gray-100;
		@apply text-gray-300;
		@apply rounded-sm;
	}

	.selected:hover {
		@apply border border-gray-400 rounded-md border-opacity-50;
	}
</style>
