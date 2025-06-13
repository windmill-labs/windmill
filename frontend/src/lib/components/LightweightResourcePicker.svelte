<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher, getContext } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { Plus, Loader2, Link2Off } from 'lucide-svelte'
	import type { AppViewerContext } from './apps/types'
	import { sendUserToast } from '$lib/toast'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import Select from './Select.svelte'

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let resourceType: string | undefined = undefined
	export let disablePortal = false
	export let expressOAuthSetup = false
	export let disabled = false

	let open = false
	let refreshCount = 0
	const appViewerContext = getContext<AppViewerContext>('AppViewerContext')

	let valueSelect =
		initialValue || value
			? {
					value: value ?? initialValue,
					label: value ?? initialValue
				}
			: undefined

	let collection = valueSelect ? [valueSelect] : []

	let loading = true
	async function loadResources(resourceType: string | undefined) {
		loading = true
		try {
			const nc = (
				await ResourceService.listResource({
					workspace: appViewerContext?.workspace ?? $workspaceStore,
					resourceType
				})
			).map((x) => ({
				value: x.path,
				label: x.path
			}))

			// TODO check if this is needed
			if (!nc.find((x) => x.value == value) && (initialValue || value)) {
				nc.push({ value: value ?? initialValue!, label: value ?? initialValue! })
			}
			collection = nc
			if (expressOAuthSetup && nc.length > 0) {
				value = nc[0].value
				valueSelect = nc[0]
			}
		} finally {
			loading = false
		}
	}

	$: $workspaceStore && loadResources(resourceType)

	$: dispatchIfMounted('change', value)

	let darkMode: boolean = false

	let drawer: Drawer | undefined = undefined

	export function askNewResource() {
		refreshCount += 1
		open = true
	}
</script>

<DarkModeObserver bind:darkMode />

{#if expressOAuthSetup}
	{#if open}
		{#key refreshCount}
			{#await import('./AppConnectLightweightResourcePicker.svelte') then Module}
				<Module.default
					workspace={appViewerContext?.workspace ?? $workspaceStore}
					{resourceType}
					express={true}
					on:error={(e) => {
						sendUserToast(e.detail, true)
						open = false
					}}
					on:refresh={async (e) => {
						await loadResources(resourceType)
						value = e.detail
						valueSelect = { value, label: value }
						open = false
					}}
				/>
			{/await}
		{/key}
	{/if}
{:else}
	<Drawer bind:this={drawer} size="800px">
		<DrawerContent
			title="Add a Resource"
			on:close={drawer.closeDrawer}
			tooltip="Resources represent connections to third party systems. Learn more on how to integrate external APIs."
			documentationLink="https://www.windmill.dev/docs/integrations/integrations_on_windmill"
		>
			{#await import('./AppConnectLightweightResourcePicker.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default
					workspace={appViewerContext?.workspace ?? $workspaceStore}
					{resourceType}
					express={false}
					on:error={(e) => {
						sendUserToast(e.detail, true)
					}}
					on:refresh={async (e) => {
						await loadResources(resourceType)
						value = e.detail
						valueSelect = { value, label: value }
						drawer?.closeDrawer?.()
						open = false
					}}
				/>
			{/await}

			<!-- <iframe
				title="App connection"
				class="w-full h-full"
				src="{base}/embed_connect?resource_type={resourceType}&workspace={appViewerContext?.workspace ??
					$workspaceStore}&express=false"
			/> -->
		</DrawerContent>
	</Drawer>
{/if}

<div class="flex flex-col w-full items-start">
	<div class="flex flex-row gap-x-1 w-full justify-between items-center">
		{#if loading}
			<Loader2 size={12} class="animate-spin" />
		{:else if expressOAuthSetup && collection.length < 2}
			{#if collection.length == 0}
				<div class="text-2xs text-primary pr-2">Connect {resourceType}</div>
			{:else if collection.length == 1 && value == collection[0]?.value}
				<div class="text-2xs text-primary pr-2 flex items-center gap-1">
					{resourceType} <div class="rounded-full w-2 h-2 bg-green-600 animate-pulse"></div>
				</div>
			{:else}
				<Button
					color="light"
					size="sm"
					on:click={() => {
						const item = collection[0]
						if (item) {
							value = item.value
							valueSelect = item
						}
					}}>Use existing</Button
				>
			{/if}
		{:else}
			<Select
				{disabled}
				{disablePortal}
				bind:value={
					() => valueSelect?.value,
					(v) => {
						value = v
						valueSelect = collection.find((x) => x.value === v)
					}
				}
				onClear={() => {
					value = undefined
					valueSelect = undefined
				}}
				clearable
				items={collection}
				class="text-clip grow min-w-0"
				placeholder="{resourceType ?? 'any'} resource"
			/>
		{/if}
		<div class="flex gap-1 items-center">
			{#if valueSelect && expressOAuthSetup}
				<Button
					{disabled}
					color="light"
					variant="contained"
					size="sm"
					btnClasses="w-8 px-0.5 py-1.5"
					iconOnly
					on:click={async () => {
						if (valueSelect && valueSelect.label) {
							value = undefined
							await ResourceService.deleteResource({
								workspace: appViewerContext?.workspace ?? $workspaceStore,
								path: valueSelect.label
							})
							await loadResources(resourceType)
							value = undefined
							valueSelect = undefined
						}
					}}
					startIcon={{ icon: Link2Off }}
				/>
			{/if}
			{#if resourceType}
				<Button
					{disabled}
					color="light"
					variant="contained"
					btnClasses="w-8 px-0.5 py-1.5"
					size="sm"
					on:click={() => {
						refreshCount += 1
						open = true
						drawer?.openDrawer?.()
					}}
					startIcon={{ icon: Plus }}
					iconOnly
				/>
			{/if}
		</div>
	</div>
</div>
