<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext, untrack } from 'svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { Plus, Loader2, Link2Off } from 'lucide-svelte'
	import type { AppViewerContext } from './apps/types'
	import { sendUserToast } from '$lib/toast'
	import Select from './select/Select.svelte'

	interface Props {
		value: string | undefined
		resourceType?: string | undefined
		disablePortal?: boolean
		expressOAuthSetup?: boolean
		disabled?: boolean
	}

	let {
		value = $bindable(),
		resourceType = undefined,
		disablePortal = false,
		expressOAuthSetup = false,
		disabled = false
	}: Props = $props()

	let open = $state(false)
	let refreshCount = $state(0)
	const appViewerContext = getContext<AppViewerContext>('AppViewerContext')

	let collection = $state(value ? [{ value, label: value }] : [])

	let loading = $state(true)
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
			if (!nc.find((x) => x.value == value) && value) {
				nc.push({ value: value, label: value })
			}
			collection = nc
			if (expressOAuthSetup && nc.length > 0) {
				value = nc[0].value
			}
		} finally {
			loading = false
		}
	}

	$effect(() => {
		$workspaceStore && resourceType && untrack(() => loadResources(resourceType))
	})

	let darkMode: boolean = $state(false)

	let drawer: Drawer | undefined = $state(undefined)

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
						}
					}}>Use existing</Button
				>
			{/if}
		{:else}
			<Select
				{disabled}
				{disablePortal}
				bind:value={
					() => value,
					(v) => {
						value = v
					}
				}
				onClear={() => {
					value = undefined
				}}
				clearable
				items={collection}
				class="text-clip grow min-w-0"
				placeholder="{resourceType ?? 'any'} resource"
			/>
		{/if}
		<div class="flex gap-1 items-center">
			{#if value && expressOAuthSetup}
				<Button
					{disabled}
					color="light"
					variant="contained"
					size="sm"
					btnClasses="w-8 px-0.5 py-1.5"
					iconOnly
					on:click={async () => {
						if (value) {
							await ResourceService.deleteResource({
								workspace: appViewerContext?.workspace ?? $workspaceStore,
								path: value
							})
							value = undefined
							await loadResources(resourceType)
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
