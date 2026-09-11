<script lang="ts">
	import { ResourceService, WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { onMount, untrack } from 'svelte'
	import AppConnect from './AppConnectDrawer.svelte'
	import ResourceEditorDrawer from './ResourceEditorDrawer.svelte'

	import { Button } from './common'
	import { Loader2, Pen, Plus, RotateCw } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'
	import Select from './select/Select.svelte'
	import ExploreAssetButton, { assetCanBeExplored } from './ExploreAssetButton.svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import { appIconComponent } from './icons'

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
		selectInputClass?: string
		class?: string
		error?: boolean
		onClear?: () => void
		excludedValues?: string[]
		datatableAsPgResource?: boolean
		workspace?: string | undefined
		disableChatOffset?: boolean
		/** Fires when this picker sets a resource, with the type it carries, and with `undefined` on
		 *  clear. Unlike an effect on `valueType` it never fires for the value the picker was opened
		 *  on, but `selectFirst` choosing the only candidate during a load does count as setting one. */
		onValueChange?: (path: string | undefined, type: string | undefined) => void
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
		selectInputClass = '',
		class: className = '',
		error = false,
		onClear = undefined,
		excludedValues = undefined,
		datatableAsPgResource = false,
		workspace = undefined,
		disableChatOffset = false,
		onValueChange = undefined
	}: Props = $props()

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	if (initialValue && value == undefined) {
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
				if (initialValue != value) {
					value = initialValue
				}
			} else {
			}
		} else {
		}
	})

	let collection = $state(value ? [{ value, label: value, type: valueType }] : [])

	export async function askNewResource() {
		appConnect?.open?.(resourceType)
	}

	export async function refreshResources() {
		await loadResources(resourceType)
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
						workspace: effectiveWorkspace,
						resourceType: rt
					})
				)
			)
			const nc = resources
				.flat()
				.filter((x) => x.resource_type != 'state' && x.resource_type != 'cache')
				.filter((x) => !excludedValues || !excludedValues.includes(x.path))
				.map((x) => ({
					value: x.path,
					label: x.path,
					type: x.resource_type
				}))

			if (datatableAsPgResource && resourceType === 'postgresql') {
				try {
					const datatables = (
						await WorkspaceService.listDataTables({
							workspace: effectiveWorkspace
						})
					).map((d) => d.name)
					for (const dt of datatables) {
						nc.push({
							value: `datatable://${dt}`,
							label: `datatable://${dt}`,
							type: 'postgresql'
						})
					}
				} catch (e) {
					console.error('Failed to load data tables', e)
				}
			}

			// TODO check if this is needed
			if (!nc.find((x) => x.value == value) && (initialValue || value)) {
				nc.push({ value: value ?? initialValue!, label: value ?? initialValue!, type: '' })
			}
			collection = nc
			if (collection.length == 1 && selectFirst && (value == undefined || value == '')) {
				value = collection[0].value
				valueType = collection[0].type
				onValueChange?.(value, valueType)
			}
		} catch (e) {
			sendUserToast('Failed to load resource types', true)
			console.error(e)
		}
		loading = false
	}

	let previousResourceType = untrack(() => resourceType)

	$effect(() => {
		effectiveWorkspace && resourceType
		untrack(() => {
			if (previousResourceType != resourceType) {
				previousResourceType = resourceType
				value = undefined
			}
		})
		untrack(() => loadResources(resourceType))
	})

	$effect(() => {
		excludedValues
		if (effectiveWorkspace && resourceType && !disabled) {
			untrack(() => loadResources(resourceType))
		}
	})

	let appConnect: AppConnect | undefined = $state()
	let resourceEditor: ResourceEditorDrawer | undefined = $state()
	let hovering = $state(false)
	let isDatatableSelected = $derived(value?.startsWith('datatable://') ?? false)
	let typeByPath = $derived(new Map(collection.map((x) => [x.value, x.type])))
	let SelectedIcon = $derived(appIconComponent(typeByPath.get(value!)))
</script>

<AppConnect
	on:refresh={async (e) => {
		await loadResources(resourceType)
		value = e.detail
		valueType = collection.find((x) => x?.value == value)?.type
		onValueChange?.(value, valueType)
	}}
	bind:this={appConnect}
	{expressOAuthSetup}
	{workspace}
	{disableChatOffset}
/>
<ResourceEditorDrawer
	bind:this={resourceEditor}
	{workspace}
	{disableChatOffset}
	on:refresh={async (e) => {
		await loadResources(resourceType)
		if (e.detail) {
			value = e.detail
			valueType = collection.find((x) => x?.value == value)?.type
			onValueChange?.(value, valueType)
			// valueSelect = { value: e.detail, label: e.detail, type: valueType ?? '' }
		}
	}}
/>
{#snippet selectedIconSnippet()}
	{#if SelectedIcon}
		<SelectedIcon height="14px" width="14px" size={14} />
	{/if}
{/snippet}

<!-- {JSON.stringify({ value, collection })} -->
<div class="flex flex-col w-full items-start {className}">
	<div
		class="flex flex-row w-full items-center relative"
		role="group"
		onmouseenter={() => (hovering = true)}
		onmouseleave={() => (hovering = false)}
	>
		<Select
			{disabled}
			{disablePortal}
			bind:value={
				() => value,
				(v) => {
					value = v
					valueType = collection.find((x) => x?.value == v)?.type
					onValueChange?.(value, valueType)
				}
			}
			onClear={() => {
				initialValue = undefined
				value = undefined
				valueType = undefined
				onValueChange?.(undefined, undefined)
				onClear?.()
			}}
			items={collection}
			clearable
			{error}
			class="text-clip grow min-w-0"
			inputClass={selectInputClass}
			placeholder={placeholder ?? `${resourceType ?? 'any'} resource`}
			itemLabelWrapperClasses="flex-1"
			id="resource-picker-select"
			inputLeadingSnippet={SelectedIcon ? selectedIconSnippet : undefined}
		>
			{#snippet startSnippet({ item })}
				{@const Icon = appIconComponent(typeByPath.get(item.value))}
				{#if Icon}
					<span class="shrink-0 text-secondary">
						<Icon height="14px" width="14px" size={14} />
					</span>
				{/if}
			{/snippet}
			{#snippet endSnippet({ item, close })}
				{#if !item.value?.startsWith('datatable://')}
					<Button
						{disabled}
						variant="subtle"
						size="xs2"
						wrapperClasses="-mr-2 pl-1 -my-2"
						btnClasses="hover:bg-surface-tertiary"
						on:click={() => (resourceEditor?.initEdit?.(item.value ?? ''), close())}
						startIcon={{ icon: Pen }}
						iconOnly
					/>
				{/if}
			{/snippet}
			{#snippet bottomSnippet({ close })}
				<div class="flex bg-surface border-t divide-x">
					{#if resourceType?.includes(',')}
						<DropdownV2
							class="flex-1 justify-stretch"
							enableFlyTransition
							items={resourceType?.split(',').map((rt) => ({
								displayName: `${rt} resource`,
								icon: Plus,
								action: () => (appConnect?.open?.(rt), close())
							})) ?? []}
						>
							{#snippet buttonReplacement()}
								<Button
									{disabled}
									color="light"
									variant="contained"
									wrapperClasses="flex-1"
									btnClasses="rounded-none"
									size="sm"
									startIcon={{ icon: Plus }}
								>
									Add a resource
								</Button>
							{/snippet}
						</DropdownV2>
					{:else}
						<Button
							{disabled}
							color="light"
							variant="contained"
							wrapperClasses="flex-1"
							btnClasses="rounded-none"
							size="sm"
							on:click={() => (appConnect?.open?.(resourceType), close())}
							startIcon={{ icon: Plus }}
						>
							Add a {resourceType} resource
						</Button>
					{/if}
					<Button
						variant="contained"
						color="light"
						btnClasses="rounded-none"
						size="sm"
						on:click={() => {
							loadResources(resourceType)
						}}
						startIcon={loading ? { icon: Loader2, classes: 'animate-spin' } : { icon: RotateCw }}
						iconOnly
					/>
				</div>
			{/snippet}
		</Select>
		{#if value && hovering && !isDatatableSelected}
			<div class="absolute {disabled ? 'right-2' : 'right-10'} z-20">
				<Button
					variant="subtle"
					size="xs2"
					wrapperClasses="pl-1"
					btnClasses="hover:bg-surface-tertiary"
					on:click={() => resourceEditor?.initEdit?.(value ?? '')}
					startIcon={{ icon: Pen }}
					iconOnly
				/>
			</div>
		{/if}
	</div>
	{#if showSchemaExplorer && value && assetCanBeExplored({ kind: 'resource', path: value }, { resource_type: resourceType })}
		<ExploreAssetButton
			class="mt-1"
			_resourceMetadata={{ resource_type: resourceType }}
			asset={{ kind: 'resource', path: value }}
			workspace={effectiveWorkspace}
		/>
	{/if}
</div>
