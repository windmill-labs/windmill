<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Editor from '$lib/components/Editor.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	let open = $state(false)
	let files: Record<string, string> = $state({})
	let version: number = $state(0)
	let editedBy: string = $state('')
	let selected: string | undefined = $state(undefined)
	let loading = $state(false)

	export async function openDrawer() {
		open = true
		await load()
	}

	async function load() {
		if (!$workspaceStore) return
		loading = true
		try {
			const res = (await WorkspaceService.getSharedUi({ workspace: $workspaceStore })) as any
			files = res.files ?? {}
			version = res.version ?? 0
			editedBy = res.edited_by ?? ''
			const keys = Object.keys(files).sort()
			if (selected === undefined || !(selected in files)) {
				selected = keys[0]
			}
		} catch (e) {
			sendUserToast(`Failed to load shared UI: ${e}`, true)
		} finally {
			loading = false
		}
	}

	const sortedPaths = $derived(Object.keys(files).sort())

	function langFromPath(p: string): any {
		const ext = (p.split('.').pop() ?? '').toLowerCase()
		if (ext === 'ts') return 'bun'
		if (ext === 'tsx') return 'tsx'
		if (ext === 'js') return 'bun'
		if (ext === 'jsx') return 'jsx'
		if (ext === 'css') return undefined
		if (ext === 'json') return 'json'
		if (ext === 'html') return undefined
		if (ext === 'md') return undefined
		return undefined
	}
</script>

<Drawer bind:open size="900px">
	<DrawerContent
		title="Shared UI folder ({sortedPaths.length} file{sortedPaths.length === 1 ? '' : 's'})"
		on:close={() => (open = false)}
		noPadding
	>
		<div class="px-3 py-2 text-xs text-tertiary border-b">
			Read-only view of the workspace's <code>ui/</code> folder. Imports of
			<code>/ui/&lt;path&gt;</code> are bundled in when you push this raw app. Edits happen via
			<code>wmill sync</code>.
			{#if version}
				Version {version}{#if editedBy}, by <code>{editedBy}</code>{/if}.
			{/if}
		</div>

		{#if loading}
			<div class="flex items-center justify-center h-full text-tertiary">Loading…</div>
		{:else if sortedPaths.length === 0}
			<div class="flex flex-col items-center justify-center h-full p-6 text-center text-tertiary">
				<div class="font-semibold mb-1">No shared UI files yet</div>
				<div class="text-sm">
					Create a <code>ui/</code> folder at the root of your sync directory, add files, then run
					<code>wmill sync push</code>.
				</div>
			</div>
		{:else}
			<Splitpanes class="!h-full">
				<Pane size={28} minSize={15}>
					<div class="overflow-auto h-full divide-y border-r">
						{#each sortedPaths as p (p)}
							<button
								class="w-full text-left px-3 py-2 text-sm font-mono hover:bg-surface-hover {selected ===
								p
									? 'bg-surface-selected'
									: ''}"
								onclick={() => (selected = p)}
							>
								<div class="truncate">{p}</div>
								<div class="text-2xs text-tertiary">
									{(files[p]?.length ?? 0).toLocaleString()} chars
								</div>
							</button>
						{/each}
					</div>
				</Pane>
				<Pane size={72}>
					{#if selected}
						{#key selected}
							<Editor
								class="flex flex-1 grow h-full"
								path={`/ui/${selected}`}
								code={files[selected] ?? ''}
								scriptLang={langFromPath(selected)}
								disabled={true}
								automaticLayout
								fixedOverflowWidgets
							/>
						{/key}
					{:else}
						<div class="flex items-center justify-center h-full text-tertiary">
							Select a file to view
						</div>
					{/if}
				</Pane>
			</Splitpanes>
		{/if}
	</DrawerContent>
</Drawer>
