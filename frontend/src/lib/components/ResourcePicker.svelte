<script lang="ts">
	import { ResourceService, type Resource } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faPlus, faRotateRight } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Button } from './common'
	import Select from 'svelte-select'

	const dispatch = createEventDispatcher()
	let resources: Resource[] = []

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let resourceType: string | undefined = undefined

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

	$: collection = resources.map((x) => ({
		value: x.path,
		label: `${x.path}${x.description ? ' | ' + x.description : ''}`
	}))
</script>

<div class="flex flex-row gap-x-1 w-full">
	<Select
		bind:justValue={value}
		items={collection}
		class="grow"
		placeholder="Pick a {resourceType} resource"
	/>
	<Button variant="border" target="_blank" size="xs" href="/resources?connect_app={resourceType}"
		><Icon scale={0.8} data={faPlus} /></Button
	>
	<Button
		variant="border"
		size="xs"
		on:click={() => {
			loadResources(resourceType)
		}}><Icon scale={0.8} data={faRotateRight} /></Button
	>
</div>
