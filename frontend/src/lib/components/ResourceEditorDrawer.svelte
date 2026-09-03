<script lang="ts">
	import { Button, Drawer } from './common'

	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import { History, Loader2, Save } from 'lucide-svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { isOwner } from '$lib/utils'
	import LocalDraftBanner from './LocalDraftBanner.svelte'
	import OpenInSessionButton from './sessions/OpenInSessionButton.svelte'
	import {
		clearPageDrawerAnchor,
		pageDrawerSessionSource,
		setPageDrawerAnchor
	} from './sessions/pageDrawerSession'
	import { RESOURCES_PATH } from './sessions/previewPaths'
	import ResourceVersionHistory from './ResourceVersionHistory.svelte'
	import IconedResourceType from './IconedResourceType.svelte'
	import { addResourceTitle } from './resourceTypeDisplay'

	let {
		workspace = undefined,
		disableChatOffset = false,
		onRestored = undefined,
		onSaved = undefined
	}: {
		workspace?: string
		disableChatOffset?: boolean
		onRestored?: () => void
		/** Fires after Save has written, for a caller showing state derived from the
		 * resource — `onRestored` only covers restoring an old version. */
		onSaved?: () => void
	} = $props()

	let drawer: Drawer | undefined = $state()
	let historyDrawer: Drawer | undefined = $state()
	let canSave = $state(true)
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
	let viewJsonSchema = $state(false)

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

	// A close reaches `on:close` on a later flush, by which point a caller that closed this drawer to
	// open another editor has already anchored the new one. Clearing then would strip that anchor.
	let keepAnchorOnClose = false

	/** Shut this drawer without going through its own close button, for a caller opening the other
	 *  editor over the same list. `keepAnchor` when that caller anchors what it opens instead. */
	export function close(opts?: { keepAnchor?: boolean }): void {
		keepAnchorOnClose = opts?.keepAnchor ?? false
		drawer?.closeDrawer?.()
	}

	/** `json` opens on the JSON editor instead of the resource type's form. For a type with a
	 *  dedicated editor elsewhere: the generic form would render its configuration field by field,
	 *  and materialize a default into every one the value leaves out. */
	export async function initEdit(p: string, opts?: { json?: boolean }): Promise<void> {
		// A `close({ keepAnchor })` on an already-closed drawer emits no close event, so the flag
		// would still be standing when the next drawer session ends and would swallow that one's
		// anchor clear. Every session starts having to clear its own.
		keepAnchorOnClose = false
		resource_type = undefined
		path = p
		selected = effectiveWorkspace
		viewJsonSchema = opts?.json ?? false
		drawer?.openDrawer?.()
		setPageDrawerAnchor(RESOURCES_PATH, p)
	}

	export async function initNew(
		resourceType: string,
		nDefaultValues?: Record<string, any>
	): Promise<void> {
		keepAnchorOnClose = false
		path = undefined
		resource_type = resourceType
		defaultValues = nDefaultValues
		selected = effectiveWorkspace
		// This drawer outlives what it opens on, so the view has to be set by every entry point
		// rather than left where the last one put it: a new resource is a typed form, whoever was
		// looking at JSON before.
		viewJsonSchema = false
		drawer?.openDrawer?.()
	}

	let mode: 'edit' | 'new' = $derived(!path ? 'new' : 'edit')

	// `selected`, not `effectiveWorkspace`: WsSpecificVersions re-points this drawer
	// at another workspace's version, and the session must act on the one shown.
	const sessionSource = $derived(
		pageDrawerSessionSource(RESOURCES_PATH, path, selected ?? effectiveWorkspace)
	)
</script>

<Drawer
	bind:this={drawer}
	size="50rem"
	{disableChatOffset}
	on:close={() => {
		if (keepAnchorOnClose) {
			keepAnchorOnClose = false
			return
		}
		clearPageDrawerAnchor(RESOURCES_PATH)
	}}
>
	<DrawerContent
		title={mode == 'edit' ? 'Edit ' + path : addResourceTitle(resource_type)}
		bannerReserved={mode == 'edit'}
		on:close={drawer?.closeDrawer}
	>
		{#snippet titleExtra()}
			{#if mode == 'new' && resource_type}
				<IconedResourceType name={resource_type} silent width="20px" height="20px" />
			{/if}
		{/snippet}
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
				bind:viewJsonSchema
				onDraftStateChange={(v) => (hasLocalDraft = v)}
				onCanWriteChange={(v) => (canWriteSelected = v)}
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
				on:click={async () => {
					// Closed before the write is awaited, the way it always was: `save()` toasts its
					// own failures and never rejects, so waiting would only add visible lag to every
					// caller of this drawer. `onSaved` still fires after the write lands.
					const saved = resourceEditor?.save()
					drawer?.closeDrawer()
					await saved
					onSaved?.()
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
