<script lang="ts">
	import { type Resource, ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import ResourceEditor from './ResourceEditor.svelte'

	const dispatch = createEventDispatcher()
	let resources: Resource[] = []

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let resourceType: string | undefined = undefined

	let resourceEditor: ResourceEditor

	async function loadResources(resourceType: string | undefined) {
		const v = value
		resources = await ResourceService.listResource({ workspace: $workspaceStore!, resourceType })
		value = v
	}
	$: {
		if ($workspaceStore) {
			loadResources(resourceType)
		}
	}
	$: dispatch('change', value)
</script>

<ResourceEditor bind:this={resourceEditor} on:refresh={() => loadResources(resourceType)} />
<select class="mt-1" bind:value placeholder="Pick a resource {resourceType}">
	<option value={undefined} />
	{#each resources as r}
		<option value={r.path}>{r.path}{r.description ? ' | ' + r.description : ''}</option>
	{/each}
</select>
<div class="flex flex-row gap-4">
	<a class="text-xs hover:underline" target="_blank" href="/resources?connect_app={resourceType}"
		>Connect the app {resourceType} to an account (if available)</a
	>
	<button
		class="text-xs text-blue-500"
		type="button"
		on:click={() => {
			resourceEditor.initNew(resourceType)
		}}
	>
		+ Create a new {resourceType} resource manually
	</button>
</div>
