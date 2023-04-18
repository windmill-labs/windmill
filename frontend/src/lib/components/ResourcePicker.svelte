<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faPen, faPlus, faRotateRight } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import Select from 'svelte-select'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'
	import AppConnect from './AppConnect.svelte'
	import { Button } from './common'
	import ResourceEditor from './ResourceEditor.svelte'

	const dispatch = createEventDispatcher()

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let resourceType: string | undefined = undefined

	let valueSelect =
		initialValue || value
			? {
					value: value ?? initialValue,
					label: value ?? initialValue
			  }
			: undefined

	console.log(valueSelect)
	let collection = [valueSelect]

	async function loadResources(resourceType: string | undefined) {
		const nc = (
			await ResourceService.listResource({
				workspace: $workspaceStore!,
				resourceType
			})
		).map((x) => ({
			value: x.path,
			label: x.path
		}))

		if (!nc.find((x) => x.value == value) && (initialValue || value)) {
			nc.push({ value: value ?? initialValue!, label: value ?? initialValue! })
		}
		collection = nc
	}

	$: {
		if ($workspaceStore) {
			loadResources(resourceType)
		}
	}
	$: dispatch('change', value)

	let appConnect: AppConnect
	let resourceEditor: ResourceEditor
</script>

<AppConnect
	on:refresh={async (e) => {
		await loadResources(resourceType)
		value = e.detail
		valueSelect = { value: e.detail, label: e.detail }
	}}
	newPageOAuth
	bind:this={appConnect}
/>

<ResourceEditor
	bind:this={resourceEditor}
	on:refresh={async (e) => {
		await loadResources(resourceType)
		if (e.detail) {
			value = e.detail
			valueSelect = { value: e.detail, label: e.detail }
		}
	}}
/>

<div class="flex flex-row gap-x-1 w-full">
	<Select
		value={valueSelect}
		on:change={(e) => {
			value = e.detail.value
			valueSelect = e.detail
		}}
		on:clear={() => {
			value = undefined
			valueSelect = undefined
		}}
		items={collection}
		class="text-clip grow min-w-0"
		placeholder="{resourceType ?? 'any'} resource"
		inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
		containerStyles={SELECT_INPUT_DEFAULT_STYLE.containerStyles}
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
