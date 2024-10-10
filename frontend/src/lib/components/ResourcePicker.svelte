<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Select from './apps/svelte-select/lib/index'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'
	import AppConnect from './AppConnectDrawer.svelte'
	import { Button } from './common'
	import DBSchemaExplorer from './DBSchemaExplorer.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Pen, Plus, RotateCw } from 'lucide-svelte'
	import ResourceEditorDrawer from './ResourceEditorDrawer.svelte'
	import { sendUserToast } from '$lib/toast'

	const dispatch = createEventDispatcher()

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let valueType: string | undefined = undefined
	export let resourceType: string | undefined = undefined
	export let disabled = false
	export let disablePortal = false
	export let showSchemaExplorer = false
	export let selectFirst = false
	export let expressOAuthSetup = false

	let valueSelect =
		initialValue || value
			? {
					value: value ?? initialValue,
					label: value ?? initialValue,
					type: valueType
			  }
			: undefined

	$: if (value === undefined && initialValue) {
		value = initialValue
	}

	let collection = valueSelect ? [valueSelect] : []

	export async function askNewResource() {
		appConnect?.open?.(resourceType, expressOAuthSetup)
	}

	let loading = true
	async function loadResources(resourceType: string | undefined) {
		loading = true
		try {
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
		} catch (e) {
			sendUserToast('Failed to load resource types', true)
			console.error(e)
		}
		loading = false
	}

	$: $workspaceStore && loadResources(resourceType)

	$: dispatch('change', value)

	let appConnect: AppConnect
	let resourceEditor: ResourceEditorDrawer

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
	bind:this={appConnect}
/>

<ResourceEditorDrawer
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
	<div class="flex flex-row w-full items-center">
		{#if collection?.length > 0}
			<Select
				{disabled}
				portal={!disablePortal}
				value={valueSelect}
				on:change={(e) => {
					value = e.detail.value
					valueType = e.detail.type
					valueSelect = e.detail
				}}
				on:clear={() => {
					initialValue = undefined
					value = undefined
					valueType = undefined
					valueSelect = undefined
					dispatch('clear')
				}}
				items={collection}
				class="text-clip grow min-w-0"
				placeholder="{resourceType ?? 'any'} resource"
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={darkMode
					? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
					: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
			/>
		{:else if !loading}
			<div class="text-2xs text-tertiary mr-2">0 found</div>
		{/if}
		{#if !loading}
			<div class="mx-0.5" />
			{#if value && value != ''}
				<Button
					{disabled}
					color="light"
					variant="contained"
					size="sm"
					btnClasses="w-8 px-0.5 py-1.5"
					on:click={() => resourceEditor?.initEdit?.(value ?? '')}
					startIcon={{ icon: Pen }}
					iconOnly
				/>
			{/if}

			{#if resourceType?.includes(',')}
				{#each resourceType.split(',') as rt}
					<Button
						{disabled}
						color="light"
						variant="contained"
						size="sm"
						btnClasses="w-8 px-0.5 py-1.5"
						on:click={() => appConnect?.open?.(rt)}
						startIcon={{ icon: Plus }}>{rt}</Button
					>
				{/each}
			{:else}
				<Button
					{disabled}
					color="light"
					variant="border"
					size="sm"
					on:click={() => appConnect?.open?.(resourceType, expressOAuthSetup)}
					startIcon={{ icon: Plus }}
					iconOnly={collection?.length > 0}
					>{#if collection?.length == 0}
						Add a {resourceType} resource
					{/if}</Button
				>
			{/if}
		{/if}

		<Button
			variant="contained"
			color="light"
			btnClasses="w-8 px-0.5 py-1.5"
			size="sm"
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
