<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Select from './apps/svelte-select/lib/index'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'
	import AppConnect from './AppConnect.svelte'
	import { Button } from './common'
	import ResourceEditor from './ResourceEditor.svelte'
	import DBSchemaExplorer from './DBSchemaExplorer.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Pen, Plus, RotateCw } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let valueType: string | undefined = undefined
	export let resourceType: string | undefined = undefined
	export let disablePortal = false
	export let showSchemaExplorer = false
	export let selectFirst = false

	let valueSelect =
		initialValue || value
			? {
					value: value ?? initialValue,
					label: value ?? initialValue,
					type: valueType
			  }
			: undefined

	let collection = valueSelect ? [valueSelect] : []

	async function loadResources(resourceType: string | undefined) {
		const nc = (
			await ResourceService.listResource({
				workspace: $workspaceStore!,
				resourceType
			})
		)
			.filter((x) => x.resource_type != 'state' && x.resource_type != 'cache')
			.map((x) => ({
				value: x.path,
				label: x.path,
				type: x.resource_type
			}))

		// TODO check if this is needed
		if (!nc.find((x) => x.value == value) && (initialValue || value)) {
			nc.push({ value: value ?? initialValue!, label: value ?? initialValue!, type: '' })
		}
		collection = nc
		if (collection.length == 1 && selectFirst && valueSelect == undefined) {
			value = collection[0].value
			valueType = collection[0].type
			valueSelect = collection[0]
		}
	}

	$: {
		if ($workspaceStore) {
			loadResources(resourceType)
		}
	}
	$: dispatch('change', value)

	let appConnect: AppConnect
	let resourceEditor: ResourceEditor

	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<AppConnect
	on:refresh={async (e) => {
		await loadResources(resourceType)
		value = e.detail
		valueType = collection.find((x) => x?.value == value)?.type
		valueSelect = {
			value: e.detail,
			label: e.detail,
			type: valueType ?? ''
		}
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
			valueType = collection.find((x) => x?.value == value)?.type
			valueSelect = { value: e.detail, label: e.detail, type: valueType ?? '' }
		}
	}}
/>
<div class="flex flex-col w-full items-start">
	<div class="flex flex-row gap-x-1 w-full">
		<Select
			portal={!disablePortal}
			value={valueSelect}
			on:change={(e) => {
				value = e.detail.value
				valueType = e.detail.type
				valueSelect = e.detail
			}}
			on:clear={() => {
				value = undefined
				valueType = undefined
				valueSelect = undefined
			}}
			items={collection}
			class="text-clip grow min-w-0"
			placeholder="{resourceType ?? 'any'} resource"
			inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
			containerStyles={darkMode
				? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
				: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
		/>

		{#if value && value != ''}
			<Button
				variant="border"
				size="xs"
				on:click={() => resourceEditor?.initEdit?.(value ?? '')}
				startIcon={{ icon: Pen }}
				iconOnly
			/>
		{/if}

		{#if resourceType?.includes(',')}
			{#each resourceType.split(',') as rt}
				<Button
					color="light"
					variant="border"
					size="xs"
					on:click={() => appConnect?.open?.(rt)}
					startIcon={{ icon: Plus }}>{rt}</Button
				>
			{/each}
		{:else}
			<Button
				color="light"
				variant="border"
				size="xs"
				on:click={() => appConnect?.open?.(resourceType)}
				startIcon={{ icon: Plus }}
				iconOnly
			/>
		{/if}

		<Button
			variant="border"
			color="light"
			size="xs"
			on:click={() => {
				loadResources(resourceType)
			}}
			startIcon={{ icon: RotateCw }}
			iconOnly
		/>
	</div>
	{#if showSchemaExplorer}
		<DBSchemaExplorer {resourceType} resourcePath={value} />
	{/if}
</div>
