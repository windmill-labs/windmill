<script lang="ts">
	import { Button, Drawer } from './common'

	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import { History, Loader2, Save } from 'lucide-svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { isOwner } from '$lib/utils'
	import LocalDraftBanner from './LocalDraftBanner.svelte'
	import OpenInSessionButton from './sessions/OpenInSessionButton.svelte'
	import { clearPageDrawerAnchor, pageDrawerSessionSource } from './sessions/pageDrawerSession'
	import { RESOURCES_PATH } from './copilot/chat/global/pageNavigation'
	import ResourceVersionHistory from './ResourceVersionHistory.svelte'

	let {
		workspace = undefined,
		disableChatOffset = false,
		onRestored = undefined
	}: { workspace?: string; disableChatOffset?: boolean; onRestored?: () => void } = $props()

	let drawer: Drawer | undefined = $state()
	let historyDrawer: Drawer | undefined = $state()
	let canSave = $state(true)
	let hasInvalidEdits = $state(false)
	let resource_type: string | undefined = $state(undefined)
	let defaultValues: Record<string, any> | undefined = $state(undefined)

	let resourceEditor:
		| {
				save: () => void
				localDraftDeployed: () => unknown
				localDraftCurrent: () => unknown
				discardLocalDraft: () => void
		  }
		| undefined = $state(undefined)
	let hasLocalDraft = $state(false)
	let canWriteSelected = $state(true)

	let path: string | undefined = $state(undefined)
	let selected: string | undefined = $state(undefined)

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)
	// The editor renders whichever workspace-specific variant `selected` points at, so history has
	// to follow it too — otherwise a restore would write over the variant the user is not looking at.
	let historyWorkspace = $derived(selected ?? effectiveWorkspace)
	// Clearing is irreversible and the backend gates it on ownership, not write access. $userStore
	// describes the user in the workspace they are signed into, so it can only answer for that one:
	// history pointed anywhere else — a ws-specific variant, or an explicit `workspace` prop — gets
	// no Clear button rather than a verdict computed from the wrong membership.
	let canClearSelected = $derived(
		historyWorkspace === $workspaceStore && isOwner(path ?? '', $userStore, $workspaceStore)
	)

	export async function initEdit(p: string): Promise<void> {
		resource_type = undefined
		path = p
		selected = effectiveWorkspace
		drawer?.openDrawer?.()
	}

	export async function initNew(
		resourceType: string,
		nDefaultValues?: Record<string, any>
	): Promise<void> {
		path = undefined
		resource_type = resourceType
		defaultValues = nDefaultValues
		selected = effectiveWorkspace
		drawer?.openDrawer?.()
	}

	let mode: 'edit' | 'new' = $derived(!path ? 'new' : 'edit')

	// `selected`, not `effectiveWorkspace`: WsSpecificVersions re-points this drawer
	// at another workspace's version, and the session must act on the one shown.
	const sessionSource = $derived.by(() => {
		const source = pageDrawerSessionSource(RESOURCES_PATH, path, selected ?? effectiveWorkspace)
		if (!source) return undefined
		return {
			...source,
			// Checked after the flush, which is what materialises the editor text: JSON that
			// does not parse never reaches `args`, so leaving would open the session on the
			// last value that did and drop the buffer with the drawer.
			beforeOpen: async () => {
				await source.beforeOpen?.()
				if (hasInvalidEdits) {
					throw new Error('This resource has changes that are not valid JSON. Fix them first.')
				}
			}
		}
	})
</script>

<Drawer
	bind:this={drawer}
	size="50rem"
	{disableChatOffset}
	on:close={() => clearPageDrawerAnchor(RESOURCES_PATH)}
>
	<DrawerContent
		title={mode == 'edit' ? 'Edit ' + path : 'Add a resource'}
		bannerReserved={mode == 'edit'}
		on:close={drawer?.closeDrawer}
	>
		{#await import('./ResourceEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				{path}
				{resource_type}
				{defaultValues}
				{workspace}
				on:refresh
				bind:this={resourceEditor}
				bind:canSave
				bind:selected
				onDraftStateChange={(v) => (hasLocalDraft = v)}
				onCanWriteChange={(v) => (canWriteSelected = v)}
				onInvalidEditsChange={(v) => (hasInvalidEdits = v)}
			/>
		{/await}
		{#snippet banner()}
			<LocalDraftBanner
				show={hasLocalDraft}
				reserveSpace={mode == 'edit'}
				getDeployed={() => resourceEditor?.localDraftDeployed()}
				getCurrent={() => resourceEditor?.localDraftCurrent()}
				onDiscard={() => resourceEditor?.discardLocalDraft()}
				disabled={!canWriteSelected}
			/>
		{/snippet}
		{#snippet actions()}
			<OpenInSessionButton source={sessionSource} />
			{#if mode == 'edit' && path && effectiveWorkspace}
				<Button
					variant="default"
					unifiedSize="md"
					startIcon={{ icon: History }}
					on:click={() => historyDrawer?.openDrawer()}
				>
					History
				</Button>
				<WsSpecificVersions
					kind="resource"
					workspaceId={effectiveWorkspace}
					initialPath={path}
					bind:selected
				/>
			{/if}
			<Button
				variant="accent"
				unifiedSize="md"
				startIcon={{ icon: Save }}
				on:click={() => {
					resourceEditor?.save()
					drawer?.closeDrawer()
				}}
				disabled={!canSave}
			>
				Save
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:this={historyDrawer} size="1200px">
	<DrawerContent title="Versions History" on:close={historyDrawer?.closeDrawer} noPadding>
		{#if path && historyWorkspace}
			<ResourceVersionHistory
				{path}
				workspace={historyWorkspace}
				canRestore={canWriteSelected}
				canClear={canClearSelected}
				onRestore={() => {
					historyDrawer?.closeDrawer()
					// Close the editor too. It holds a baseline captured before the restore, and
					// any local draft on top of it, so saving from it afterwards would write the
					// pre-restore value straight back over the version just restored.
					drawer?.closeDrawer()
					// Its own callback rather than the `refresh` event: callers bind that to
					// reopening a picker (EditorBar), which a restore should not trigger.
					onRestored?.()
				}}
			/>
		{/if}
	</DrawerContent>
</Drawer>
