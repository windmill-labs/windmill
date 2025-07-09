<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onMount, untrack } from 'svelte'
	import AppConnect from './AppConnectDrawer.svelte'
	import ResourceEditorDrawer from './ResourceEditorDrawer.svelte'

	import { Button } from './common'
	import DBManagerDrawerButton from './DBManagerDrawerButton.svelte'
	import { Pen, Plus, RotateCw } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import { isDbType } from './apps/components/display/dbtable/utils'
	import Select from './select/Select.svelte'

	interface Props {
		initialValue?: string | undefined
		value?: string | undefined
		valueType?: string | undefined
		resourceType?: string | undefined
		disabled?: boolean
		disablePortal?: boolean
		showSchemaExplorer?: boolean
		selectFirst?: boolean
		expressOAuthSetup?: boolean
		defaultValues?: Record<string, any> | undefined
		placeholder?: string | undefined
		onClear?: () => void
	}

	let {
		initialValue = $bindable(undefined),
		value = $bindable(undefined),
		valueType = $bindable(undefined),
		resourceType = undefined,
		disabled = false,
		disablePortal = false,
		showSchemaExplorer = false,
		selectFirst = false,
		expressOAuthSetup = false,
		defaultValues = undefined,
		placeholder = undefined,
		onClear = undefined
	}: Props = $props()

	if (initialValue && value == undefined) {
		console.log('initialValue', initialValue)
		value = initialValue
	}

	onMount(() => {
		setTimeout(() => {
			if (Object.keys(defaultValues ?? {}).length > 0 && resourceType) {
				resourceEditor?.initNew?.(resourceType, defaultValues)
			}
		}, 500)
	})

	// let valueSelect = $state(
	// 	initialValue || value
	// 		? {
	// 				value: value ?? initialValue,
	// 				label: value ?? initialValue,
	// 				type: valueType
	// 			}
	// 		: undefined
	// )

	$effect(() => {
		if (value === undefined) {
			if (initialValue) {
				console.log('initialValue', initialValue)
				value = initialValue
			} else {
				console.log('no value')
			}
		} else {
			console.log('value', value)
		}
	})

	let collection = $state(value ? [{ value, label: value, type: valueType }] : [])

	export async function askNewResource() {
		appConnect?.open?.(resourceType)
	}

	let loading = $state(true)
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
			if (collection.length == 1 && selectFirst && value == undefined) {
				console.log('selectFirst', collection[0].value)
				value = collection[0].value
				valueType = collection[0].type
			}
		} catch (e) {
			sendUserToast('Failed to load resource types', true)
			console.error(e)
		}
		loading = false
	}

	let oldResourceType = resourceType
	$effect(() => {
		$workspaceStore && resourceType
		if (resourceType != oldResourceType) {
			oldResourceType = resourceType
			value = undefined
		}
		untrack(() => loadResources(resourceType))
	})

	let appConnect: AppConnect | undefined = $state()
	let resourceEditor: ResourceEditorDrawer | undefined = $state()
</script>

<AppConnect
	on:refresh={async (e) => {
		await loadResources(resourceType)
		value = e.detail
		valueType = collection.find((x) => x?.value == value)?.type
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
			// valueSelect = { value: e.detail, label: e.detail, type: valueType ?? '' }
		}
	}}
/>
<!-- {JSON.stringify({ value, collection })} -->
<div class="flex flex-col w-full items-start min-h-9">
	<div class="flex flex-row w-full items-center">
		{#if collection?.length > 0}
			<Select
				{disabled}
				{disablePortal}
				bind:value={
					() => value,
					(v) => {
						value = v
						valueType = collection.find((x) => x?.value == v)?.type
					}
				}
				onClear={() => {
					initialValue = undefined
					value = undefined
					valueType = undefined
					onClear?.()
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
