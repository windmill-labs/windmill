<script lang="ts">
	import { goto } from '$app/navigation'
	import { faPlus, faScroll } from '@fortawesome/free-solid-svg-icons'
	import Fuse from 'fuse.js'
	import { Script } from '$lib/gen'
	import { ScriptService } from '$lib/gen'
	import { workspaceStore, hubScripts } from '$lib/stores'
	import { ButtonPopup, ButtonPopupItem } from '$lib/components/common'
	import type { HubItem } from '../flows/pickers/model'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Icon from 'svelte-awesome'

	const drawers: {
		template: Drawer | undefined
	} = {
		template: undefined
	}

	let hubItems: HubItem[]
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
	<ButtonPopup size="sm" spacingSize="xl" startIcon={{ icon: faPlus }} href="/scripts/add">
		<svelte:fragment slot="main">New Script</svelte:fragment>
		<ButtonPopupItem on:click={() => drawers.template?.toggleDrawer()}>
			Import from template
		</ButtonPopupItem>
	</ButtonPopup>
</div>

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
