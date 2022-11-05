<script lang="ts">
	import { goto } from '$app/navigation'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import Fuse from 'fuse.js'
	import { Script } from '$lib/gen'
	import { ScriptService } from '$lib/gen'
	import { workspaceStore, hubScripts } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import { Button, ButtonPopup, ButtonPopupItem } from '$lib/components/common'
	import ItemPicker from '../ItemPicker.svelte'
	import type { HubItem } from '../flows/pickers/model'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { flowStore, initFlow } from '$lib/components/flows/flowStore'

	const drawers: {
		hub: ItemPicker | undefined
		template: Drawer | undefined
		json: Drawer | undefined
	} = {
		hub: undefined,
		template: undefined,
		json: undefined
	}
	let hubItems: HubItem[]
	let pendingJson: string
	let templateScripts: Script[] = []
	let templateFilter = ''
	let filteredTemplates: Script[] | undefined
	const fuseOptions = {
		includeScore: false,
		keys: ['description', 'path', 'content', 'hash', 'summary']
	}
	const templateFuse: Fuse<Script> = new Fuse(templateScripts, fuseOptions)

	$: hubItems = $hubScripts?.filter((x) => x.kind == Script.kind.SCRIPT) || []

	$: filteredTemplates =
		templateFilter.length > 0
			? templateFuse.search(templateFilter).map((value) => value.item)
			: templateScripts

	function importJson() {
		Object.assign($flowStore, JSON.parse(pendingJson))

		initFlow($flowStore)
		sendUserToast('OpenFlow imported from JSON')
		drawers.json?.toggleDrawer()
	}

	async function loadTemplateScripts(): Promise<void> {
		templateScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			isTemplate: true
		})
		templateFuse.setCollection(templateScripts)
	}
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<ButtonPopup size="sm" startIcon={{ icon: faPlus }} href="/scripts/add">
		<svelte:fragment slot="main">New script</svelte:fragment>
		<ButtonPopupItem on:click={() => drawers.hub?.openDrawer()}>
			Import script from WindmillHub
		</ButtonPopupItem>
		<ButtonPopupItem on:click={() => drawers.template?.toggleDrawer()}>
			Import script from template
		</ButtonPopupItem>
		<ButtonPopupItem on:click={() => drawers.json?.toggleDrawer()}>
			Import script from raw JSON
		</ButtonPopupItem>
	</ButtonPopup>
</div>

<!-- Initially hidden elements in a drawer -->
<!-- WindmillHub script list -->
<ItemPicker
	bind:this={drawers.hub}
	pickCallback={(path) => {
		console.log('pick', { path })
		goto('/scripts/add?hub=' + path)
	}}
	itemName={'Script'}
	extraField="summary"
	loadItems={async () => {
		return hubItems
	}}
/>
<!-- Template script list -->
<Drawer bind:this={drawers.template} size="800px" on:open={loadTemplateScripts}>
	<DrawerContent title="Pick a template" on:close={() => drawers.template?.toggleDrawer()}>
		<div class="pt-2 pb-4">
			<input placeholder="Search templates" bind:value={templateFilter} class="search-bar" />
		</div>
		<div class="flex flex-col mb-2 md:mb-6">
			{#if filteredTemplates && filteredTemplates.length > 0}
				{#each filteredTemplates as { summary, path, hash }}
					<a
						class="p-1 flex flex-row items-baseline gap-2 selected text-gray-700"
						href="/scripts/add?template={path}"
					>
						{#if summary}
							<p class="text-sm font-semibold">{summary}</p>
						{/if}

						<p class="text-sm">{path}</p>
						<p class="text-gray-400 text-xs text-right grow">
							Last version: {hash}
						</p>
					</a>
				{/each}
			{:else}
				<p class="text-sm text-gray-700">No templates</p>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
<!-- Raw JSON -->
<Drawer bind:this={drawers.json} size="800px">
	<DrawerContent title="Import JSON" on:close={() => drawers.json?.toggleDrawer()}>
		<div class="p-2"><Button size="sm" on:click={importJson}>Import</Button></div>
		<SimpleEditor bind:code={pendingJson} lang="json" class="h-full" />
	</DrawerContent>
</Drawer>
