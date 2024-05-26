<script lang="ts">
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import Select from './apps/svelte-select/lib/index'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../defaults'

	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { Plus } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	export let initialValue: string | undefined = undefined
	export let value: string | undefined = initialValue
	export let resourceType: string | undefined = undefined
	export let disablePortal = false

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

	let iframe: HTMLIFrameElement | undefined = undefined

	function processEvent(event: MessageEvent) {
		if (event.origin !== window.location.origin) {
			return
		}

		if (event.data.type === 'refresh') {
			value = event.data.detail
			valueSelect = { value, label: value }
			drawer?.closeDrawer?.()
		}
	}
</script>

<DarkModeObserver bind:darkMode />

<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title="Add a Resource"
		on:close={drawer.closeDrawer}
		tooltip="Resources represent connections to third party systems. Learn more on how to integrate external APIs."
		documentationLink="https://www.windmill.dev/docs/integrations/integrations_on_windmill"
	>
		<iframe
			bind:this={iframe}
			title="App connection"
			class="w-full h-full"
			src="/embed_connect?resource_type={resourceType}"
		/>
	</DrawerContent>
</Drawer>

<div class="flex flex-col w-full items-start">
	<div class="flex flex-row gap-x-1 w-full">
		<Select
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
				color="light"
				variant="border"
				size="xs"
				on:click={() => {
					window.removeEventListener('message', processEvent)
					window.addEventListener('message', processEvent, {
						once: true
					})

					drawer?.openDrawer?.()
				}}
				startIcon={{ icon: Plus }}
				iconOnly
			/>
		{/if}
	</div>
</div>
