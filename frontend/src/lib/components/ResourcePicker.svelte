<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher, onMount } from 'svelte'
	import AppConnect from './AppConnectDrawer.svelte'
	import ResourceEditorDrawer from './ResourceEditorDrawer.svelte'

	import { Button } from './common'
	import DBManagerDrawerButton from './DBManagerDrawerButton.svelte'
	import { Pen, Plus, RotateCw } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { isDbType } from './apps/components/display/dbtable/utils'
	import Select from './select/Select.svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let valueType: string | undefined = undefined
	export let resourceType: string | undefined = undefined
	export let disabled = false
	export let disablePortal = false
	export let showSchemaExplorer = false
	export let selectFirst = false
	export let expressOAuthSetup = false
	export let defaultValues: Record<string, any> | undefined = undefined
	export let placeholder: string | undefined = undefined

	onMount(() => {
		setTimeout(() => {
			if (Object.keys(defaultValues ?? {}).length > 0 && resourceType) {
				resourceEditor?.initNew?.(resourceType, defaultValues)
			}
		}, 500)
	})

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
		appConnect?.open?.(resourceType)
	}

	let loading = true
	async function loadResources(resourceType: string | undefined) {
		loading = true
		try {
			const resourceTypesToQuery =
				resourceType === 'snowflake' ? ['snowflake', 'snowflake_oauth'] : [resourceType]

			const resources = await Promise.all(
				resourceTypesToQuery.map((rt) =>
					ResourceService.listResource({
						workspace: $workspaceStore!,
						resourceType: rt
					})
				)
			)
			const nc = resources
				.flat()
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

	$: dispatchIfMounted('change', value)

	let appConnect: AppConnect
	let resourceEditor: ResourceEditorDrawer
</script>

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
	{expressOAuthSetup}
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

<div class="flex flex-col w-full items-start min-h-9">
	<div class="flex flex-row w-full items-center">
		{#if collection?.length > 0}
			<Select
				{disabled}
				{disablePortal}
				bind:value={
					() => valueSelect?.value,
					(v) => {
						valueSelect = collection.find((x) => x.value === v)
						value = v
						valueType = valueSelect?.type
					}
				}
				onClear={() => {
					initialValue = undefined
					value = undefined
					valueType = undefined
					valueSelect = undefined
					dispatch('clear')
				}}
				items={collection}
				clearable
				class="text-clip grow min-w-0"
				placeholder={placeholder ?? `${resourceType ?? 'any'} resource`}
			/>
		{:else if !loading}
			<div class="text-2xs text-tertiary mr-2">0 found</div>
		{/if}
		{#if !loading}
			<div class="mx-0.5"></div>
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
					on:click={() => appConnect?.open?.(resourceType)}
					startIcon={{ icon: Plus }}
					iconOnly={collection?.length > 0}
					>{#if collection?.length == 0}
						Add a {resourceType} resource
					{/if}</Button
				>
				<div class="mx-0.5"></div>
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
	{#if showSchemaExplorer && isDbType(resourceType) && value}
		<DBManagerDrawerButton {resourceType} resourcePath={value} />
	{/if}
</div>
