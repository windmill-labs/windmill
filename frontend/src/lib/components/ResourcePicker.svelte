<script lang="ts">
	import { ResourceService, type Resource } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faPen, faPlus, faRotateRight } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Button } from './common'
	import Select from 'svelte-select'
	import AppConnect from './AppConnect.svelte'
	import ResourceEditor from './ResourceEditor.svelte'
	import { truncate } from '$lib/utils'

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
		label: x.path
	}))
	let appConnect: AppConnect
	let resourceEditor: ResourceEditor
</script>

<AppConnect
	on:refresh={async (e) => {
		await loadResources(resourceType)
		value = e.detail
	}}
	newPageOAuth
	bind:this={appConnect}
/>

<ResourceEditor
	bind:this={resourceEditor}
	on:refresh={async (e) => {
		await loadResources(resourceType)
		console.log(e)
		if (e.detail) {
			value = e.detail
		}
	}}
/>

<div class="flex flex-row gap-x-1 w-full">
	<Select
		listAutoWidth={false}
		--height="34px"
		value={collection.find((x) => x.value == value)}
		bind:justValue={value}
		items={collection}
		class="text-clip grow min-w-0"
		placeholder="{resourceType} resource"
	/>
	{#if value && value != ''}
		<Button variant="border" size="xs" on:click={() => resourceEditor?.initEdit?.(value ?? '')}>
			<Icon scale={0.8} data={faPen} /></Button
		>
	{/if}

	<Button variant="border" size="xs" on:click={() => appConnect?.open?.(resourceType)}>
		<Icon scale={0.8} data={faPlus} /></Button
	>
	<Button
		variant="border"
		size="xs"
		on:click={() => {
			loadResources(resourceType)
		}}><Icon scale={0.8} data={faRotateRight} /></Button
	>
</div>

<style>
	:global(.svelte-select-list) {
		font-size: small !important;
	}
</style>
