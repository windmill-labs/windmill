<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher, getContext } from 'svelte'
	import Select from './apps/svelte-select/lib/index'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'

	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { Plus, Loader2 } from 'lucide-svelte'
	import type { AppViewerContext } from './apps/types'
	import { sendUserToast } from '$lib/toast'

	const dispatch = createEventDispatcher()

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
			: ''

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

		// TODO check if this is needed
		if (!nc.find((x) => x.value == value) && (initialValue || value)) {
			nc.push({ value: value ?? initialValue!, label: value ?? initialValue! })
		}
		collection = nc
	}

	$: $workspaceStore && loadResources(resourceType)

	$: dispatch('change', value)

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
			{#await import('./AppConnectLightweightResourcePicker.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default
					workspace={appViewerContext?.workspace ?? $workspaceStore}
					{resourceType}
					express={true}
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
					on:refresh={(e) => {
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
	<div class="flex flex-row gap-x-1 w-full">
		<Select
			{disabled}
			portal={!disablePortal}
			value={valueSelect}
			on:change={(e) => {
				value = e.detail.value
				valueSelect = e.detail
			}}
			on:clear={() => {
				value = undefined
				valueSelect = ''
			}}
			items={collection}
			class="text-clip grow min-w-0"
			placeholder="{resourceType ?? 'any'} resource"
			inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
			containerStyles={darkMode
				? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
				: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
		/>

		{#if resourceType}
			<Button
				{disabled}
				color="light"
				variant="contained"
				btnClasses="w-8 px-0.5 py-1.5"
				size="sm"
				on:click={() => {
					open = true
					drawer?.openDrawer?.()
				}}
				startIcon={{ icon: Plus }}
				iconOnly
			/>
		{/if}
	</div>
</div>
