<script lang="ts">
	import { VariableService, WorkspaceService } from '$lib/gen'
	import { createEventDispatcher, untrack } from 'svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import OpenInSessionButton from './sessions/OpenInSessionButton.svelte'
	import {
		clearPageDrawerAnchor,
		pageDrawerSessionSource,
		setPageDrawerAnchor
	} from './sessions/pageDrawerSession'
	import { VARIABLES_PATH } from './sessions/previewPaths'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import { ArrowLeft, Save } from 'lucide-svelte'
	import VariableForm from './VariableForm.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { resource } from 'runed'
	import { getUserExt } from '$lib/user'
	import type { UserExt } from '$lib/stores'
	import {
		UserDraft,
		draftValuesEqual,
		flushDraftDelete,
		settleDraftAfterWrite,
		type UserDraftHandle
	} from '$lib/userDraft.svelte'
	import LocalDraftBanner from './LocalDraftBanner.svelte'
	import { isEncryptedDraftValue } from '$lib/encryptedDraft'
	import { setLocalDraftHint } from '$lib/localDraftHints.svelte'

	const dispatch = createEventDispatcher()

	type VariableState = {
		path: string
		variable: { value: string; is_secret: boolean; description: string }
		labels: string[] | undefined
		wsSpecific: boolean
	}

	// The "current" workspace this editor defaults New/Edit actions to. Session
	// editors pass their acting workspace so secrets are created/updated there
	// rather than in the navigation workspace. Defaults to $workspaceStore.
	let {
		workspace = undefined,
		useDrawer = true,
		onBack = undefined,
		onSaved = undefined,
		onRemoved = undefined
	}: {
		workspace?: string
		/**
		 * False renders the editor in place instead of in a drawer, for a host that
		 * gives it a pane of its own (a session's variable tab). Same convention as
		 * the trigger editors. `editVariable` still selects what is shown; there is
		 * no drawer to open, so it simply takes effect.
		 */
		useDrawer?: boolean
		/** Inline only: offered in the header when the host replaced something the
		 * user should be able to get back to (a session tab that took over the list). */
		onBack?: () => void
		/** Fires after a save, with the path it wrote to — which is not the one it was
		 * opened on when the user renamed it — and the workspace and path it started on. */
		onSaved?: (savedPath?: string, fromPath?: string, fromWorkspace?: string) => void
		/** The variable is gone — a draft-only one whose draft was discarded — with the
		 * workspace and path it was showing. A host addressing it by those (a session
		 * tab) has to stop showing it. */
		onRemoved?: (fromPath: string, fromWorkspace: string) => void
	} = $props()
	let curWs = $derived(workspace ?? $workspaceStore)

	let editPath: string | undefined = $state(undefined)

	// Per-workspace handles are driven by `useMany`. We track the workspace
	// IDs (and their seeded defaults) in a parallel `$state` array; on every
	// mutation `useMany` reconciles, acquiring entries for new workspaces and
	// releasing them on component teardown. `states` indexes the resulting
	// handles by workspace ID for ergonomic lookup downstream.
	let workspaceSpecs = $state<Array<{ ws: string; defaultValue: VariableState }>>([])
	let initialStates: Record<string, VariableState> = $state({})
	let existedInitially: Record<string, boolean> = $state({})
	let extraPerms: Record<string, Record<string, boolean>> = $state({})
	let perWsUser: Record<string, UserExt | undefined> = $state({})
	let selected: string | undefined = $state(undefined)
	let pathError = $state('')

	const handlesArray = UserDraft.useMany<VariableState>(() =>
		workspaceSpecs.map((s) => ({
			itemKind: 'variable' as const,
			path: editPath ?? '',
			workspace: s.ws,
			defaultValue: s.defaultValue,
			// Autosaves landing back on the deployed value become deletes (same
			// comparison as the banner's `dirtyWorkspaces`, so they can't disagree).
			// Guarded by `existedInitially` so draft-only items aren't destroyed.
			discardIf: (val) => !!existedInitially[s.ws] && draftValuesEqual(val, initialStates[s.ws])
		}))
	)
	const states = $derived.by(() => {
		const out: Record<string, UserDraftHandle<VariableState>> = {}
		for (let i = 0; i < workspaceSpecs.length; i++) {
			const handle = handlesArray[i]
			if (handle) out[workspaceSpecs[i].ws] = handle
		}
		return out
	})

	/** Register a workspace so `useMany` acquires (or reuses) its handle.
	 * `defaultValue` is what the handle reports when no autosave is persisted;
	 * an existing autosave always wins. The default itself never round-trips
	 * to localStorage — only the user's first real edit triggers a write. */
	function ensureHandle(ws: string, defaultValue: VariableState): void {
		if (workspaceSpecs.some((s) => s.ws === ws)) return
		workspaceSpecs.push({ ws, defaultValue })
	}

	let drawer: Drawer | undefined = $state()
	let form: VariableForm | undefined = $state()

	const deployTo = resource(
		() => selected,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)

	const MAX_VARIABLE_LENGTH = 10000
	const edit = $derived(editPath !== undefined)
	const initialPath = $derived(editPath ?? '')
	// `selected`, not `curWs`: WsSpecificVersions re-points this drawer at another
	// workspace's version of the variable, and the session must act on the one the
	// user is looking at.
	const sessionSource = $derived(
		pageDrawerSessionSource(VARIABLES_PATH, editPath, selected ?? curWs)
	)
	const current = $derived(selected ? states[selected]?.draft : undefined)
	const can_write = $derived.by(() => {
		if (!selected || !edit) return true
		const perms = extraPerms[selected]
		if (!perms) return true
		return canWrite(editPath ?? '', perms, perWsUser[selected] ?? $userStore)
	})
	const dirtyWorkspaces = $derived(
		Object.keys(states).filter((ws) => !draftValuesEqual(states[ws].draft, initialStates[ws]))
	)

	// The list-page `*` hint is owned by UserDraftDbSyncer (set on save, cleared
	// on delete). The editor only CLEARS it — a workspace at the deployed
	// baseline has no draft, so drop any stale hint (this is how a draft
	// discarded in another tab vanishes on reopen). Never SET here.
	$effect(() => {
		const p = editPath
		const loadedWs = Object.keys(states)
		const dirty = dirtyWorkspaces
		untrack(() => {
			if (!p) return
			for (const ws of loadedWs) {
				if (!dirty.includes(ws)) setLocalDraftHint(ws, 'variable', p, false)
			}
		})
	})
	const anyDirty = $derived(dirtyWorkspaces.length > 0)
	// Banner is scoped to the selected workspace — the diff/discard only
	// operate on it, so showing it for an unrelated dirty workspace would be
	// misleading. The cross-workspace `otherDirty` alert below still covers
	// that case.
	const selectedDirty = $derived(!!selected && dirtyWorkspaces.includes(selected))
	const otherDirty = $derived(
		dirtyWorkspaces.length == 1 ? dirtyWorkspaces.filter((ws) => ws !== curWs) : dirtyWorkspaces
	)
	const dirtyValid = $derived(
		dirtyWorkspaces.every((ws) => {
			const v = states[ws].draft
			// `$encrypted:` markers are ciphertext; the backend re-derives the
			// real value on save, so the length cap doesn't apply.
			return (
				!!v &&
				(isEncryptedDraftValue(v.variable.value) || v.variable.value.length <= MAX_VARIABLE_LENGTH)
			)
		})
	)
	const dirtyCanWrite = $derived(
		dirtyWorkspaces.every((ws) => {
			const perms = extraPerms[ws]
			return !perms || canWrite(editPath ?? '', perms, perWsUser[ws] ?? $userStore)
		})
	)

	// Lazy-fetch the variable for the selected workspace when not already cached
	$effect(() => {
		const ws = selected
		const p = editPath
		if (!ws || !p) return
		if (ws in states) return
		untrack(() => {
			Promise.all([
				VariableService.getVariable({
					workspace: ws,
					path: p,
					decryptSecret: false,
					getDraft: true
				}),
				getUserExt(ws)
			]).then(([v, user]) => {
				// `.draft` already holds the editor's `VariableState` shape.
				const savedDraftState = (v as any).draft as VariableState | undefined
				// Deployed baseline as the dirty-check reference, so the banner
				// compares draft-vs-deployed and fires immediately when a draft exists.
				const deployedState: VariableState = {
					path: v.path,
					variable: {
						value: v.value ?? '',
						is_secret: v.is_secret,
						description: v.description ?? ''
					},
					labels: v.labels ?? undefined,
					wsSpecific: v.ws_specific ?? false
				}
				// Open with the saved draft if present, else the deployed.
				const s: VariableState = savedDraftState ?? deployedState
				ensureHandle(ws, s)
				initialStates[ws] = structuredClone(deployedState)
				// Draft-only paths (`no_deployed`) have no row — saving must
				// CREATE, not update (update 404s).
				existedInitially[ws] = !(v as any).no_deployed
				extraPerms[ws] = v.extra_perms ?? {}
				perWsUser[ws] = user
			})
		})
	})

	function reset() {
		// Clearing workspaceSpecs triggers useMany's reconcile to release
		// every acquired entry. The $derived `states` then collapses to {}.
		workspaceSpecs = []
		initialStates = {}
		existedInitially = {}
		extraPerms = {}
		perWsUser = {}
		pathError = ''
	}

	export function initNew(): void {
		reset()
		editPath = undefined
		const ws = curWs!
		const s: VariableState = {
			path: '',
			variable: { value: '', is_secret: true, description: '' },
			labels: undefined,
			wsSpecific: false
		}
		ensureHandle(ws, s)
		initialStates[ws] = structuredClone(s)
		existedInitially[ws] = false
		selected = ws
		drawer?.openDrawer()
	}

	export function editVariable(edit_path: string): void {
		reset()
		editPath = edit_path
		selected = curWs!
		drawer?.openDrawer()
		setPageDrawerAnchor(VARIABLES_PATH, edit_path)
	}

	async function loadSecret(): Promise<void> {
		if (!editPath || !selected) return
		const getV = await VariableService.getVariable({
			workspace: selected,
			path: editPath,
			decryptSecret: true
		})
		const s = states[selected]?.draft
		const ini = initialStates[selected]
		if (s) s.variable.value = getV.value ?? ''
		if (ini) ini.variable.value = getV.value ?? ''
		form?.setCode(getV.value ?? '')
	}

	async function save(): Promise<void> {
		// Everything the writes need, read before the first await. An inline host can
		// re-point this editor at another variable mid-flight, and every one of these
		// would then be that variable's: the writes would send its state under this
		// one's path, and the baseline below would overwrite its own.
		const from = editPath
		const fromWs = curWs
		const payloads = dirtyWorkspaces.map((ws) => ({
			ws,
			s: $state.snapshot(states[ws].draft!) as VariableState,
			ini: $state.snapshot(initialStates[ws]) as VariableState,
			existed: !!existedInitially[ws]
		}))
		// The path the ACTING workspace's write used. `WsSpecificVersions` can point
		// the form at a linked workspace, and a rename made there is that workspace's
		// alone — reporting it would move a host that is looking at this one.
		const savedPath = payloads.find((pl) => pl.ws === fromWs)?.s.path ?? from
		// Set once the acting workspace's own write has committed. The workspaces are
		// written in sequence and a later one throwing aborts the rest, but what this
		// one wrote is already deployed — a host told nothing would stay pointed at a
		// path it has moved off.
		let committed: string | undefined = undefined
		// Follow the rename locally and tell the host, for whatever committed. Guarded
		// on this editor still being the one that was saved: re-pointed, `editPath` is
		// the variable it moved to. `close` only when every workspace is done — after a
		// partial failure the drawer is where the one that still needs saving is retried.
		const reportSaved = (saved: string | undefined, close: boolean) => {
			if (editPath === from) {
				if (saved && saved !== editPath) editPath = saved
				if (close) drawer?.closeDrawer()
			}
			onSaved?.(saved, from, fromWs)
		}
		try {
			for (const { ws, s, ini, existed } of payloads) {
				if (existed) {
					await VariableService.updateVariable({
						workspace: ws,
						path: ini.path,
						requestBody: {
							path: ini.path != s.path ? s.path : undefined,
							value: s.variable.value == '' ? undefined : s.variable.value,
							is_secret:
								ini.variable.is_secret != s.variable.is_secret ? s.variable.is_secret : undefined,
							description:
								ini.variable.description != s.variable.description
									? s.variable.description
									: undefined,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
				} else {
					await VariableService.createVariable({
						workspace: ws,
						requestBody: {
							path: s.path,
							value: s.variable.value,
							is_secret: s.variable.is_secret,
							description: s.variable.description,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
				}
				// Per workspace, as each write lands: a later one throwing must not leave
				// this one's draft dropped below a baseline that never moved, which reads
				// as dirty and retries the create it already made.
				if (editPath === from) {
					initialStates[ws] = s
					existedInitially[ws] = true
				}
				// The just-saved state is the new deployed baseline; `settleDraftAfterWrite`
				// resets the handle to it via `discard` (not `remove` — blanking the cell
				// to `undefined` reads as dirty), and keeps an edit made mid-request. Each
				// workspace settles against the path its own write used, not the reported one.
				if (ws === fromWs) committed = s.path
				settleDraftAfterWrite('variable', s, states[ws]?.draft, from ?? '', s.path, {
					workspace: ws
				})
				// Path now exists server-side — drop the autocomplete cache so
				// it shows up immediately instead of after the 60s TTL.
				invalidateWorkspacePaths(ws)
			}
			sendUserToast(
				edit ? `Updated variable in ${payloads.length} workspace(s)` : `Created variable`
			)
			dispatch('create')
			reportSaved(savedPath, true)
		} catch (err) {
			sendUserToast(`Could not save variable: ${err.body}`, true)
			// The workspaces that did not get this far keep their drafts under the path
			// they still hold the item at; re-keying them onto this rename would move a
			// path they never wrote. They stay listed, and retryable from here.
			if (committed) reportSaved(committed, false)
		}
	}
</script>

{#snippet draftBanner()}
	<LocalDraftBanner
		show={edit && selectedDirty}
		reserveSpace={edit}
		getDeployed={() => (selected ? initialStates[selected] : undefined)}
		getCurrent={() => current}
		onDiscard={async () => {
			if (!selected) return
			const ws = selected
			const from = editPath ?? ''
			const fromWs = curWs
			UserDraft.discard('variable', from, initialStates[ws], { workspace: ws })
			// A draft-only variable has no deployed row under the draft, so discarding
			// it removed the variable: `initialStates` holds a synthesized stand-in, not
			// a baseline to fall back to. Only once the delete has landed — until then
			// the variable is still there, and a host would leave on a row it can see.
			const landed = await flushDraftDelete('variable', from, { workspace: ws })
			if (!existedInitially[ws] && landed && from && fromWs) onRemoved?.(from, fromWs)
		}}
		disabled={!can_write}
	/>
{/snippet}

{#snippet editorActions()}
	<!-- Only the drawer offers the hand-off: rendered inline the editor is
	     already inside the session it would open. -->
	{#if useDrawer}
		<OpenInSessionButton source={sessionSource} />
	{/if}
	{#if edit && curWs}
		<WsSpecificVersions kind="variable" workspaceId={curWs} {initialPath} bind:selected />
	{/if}
	<Button
		on:click={save}
		disabled={!anyDirty || !dirtyValid || !dirtyCanWrite || pathError != ''}
		startIcon={{ icon: Save }}
		variant="accent"
		unifiedSize="sm"
	>
		{edit ? 'Update' : 'Save'}
	</Button>
{/snippet}

{#snippet editorBody()}
	<div class="flex flex-col gap-8 pb-2">
		{#if !can_write}
			<Alert type="warning" title="Only read access">
				You only have read access to this resource and cannot edit it
			</Alert>
		{/if}

		{#if otherDirty.length > 0}
			<Alert type="warning" title="Editing multiple workspaces">
				You are going to edit the value in: {otherDirty.join(', ')}
			</Alert>
		{/if}

		{#if current}
			{#key current}
				<VariableForm
					bind:this={form}
					bind:path={current.path}
					bind:pathError
					bind:variable={current.variable}
					bind:labels={current.labels}
					bind:wsSpecific={current.wsSpecific}
					{initialPath}
					deployTo={deployTo.current}
					{can_write}
					{edit}
					onLoadSecret={loadSecret}
					{workspace}
				/>
			{/key}
		{/if}
	</div>
{/snippet}

{#if useDrawer}
	<Drawer bind:this={drawer} size="50rem" on:close={() => clearPageDrawerAnchor(VARIABLES_PATH)}>
		<DrawerContent
			title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
			bannerReserved={edit}
			on:close={drawer?.closeDrawer}
		>
			{#snippet banner()}
				{@render draftBanner()}
			{/snippet}
			{@render editorBody()}
			{#snippet actions()}
				{@render editorActions()}
			{/snippet}
		</DrawerContent>
	</Drawer>
{:else}
	<div class="flex flex-col h-full min-h-0">
		<div class="flex flex-row items-center gap-2 justify-between px-4 py-2 border-b">
			<div class="flex flex-row items-center gap-2 min-w-0">
				{#if onBack}
					<Button
						variant="subtle"
						unifiedSize="sm"
						startIcon={{ icon: ArrowLeft }}
						on:click={onBack}
						title="Back to Variables"
						iconOnly
					/>
				{/if}
				<span class="text-sm font-semibold truncate"
					>{edit ? `Update variable at ${initialPath}` : 'Add a variable'}</span
				>
			</div>
			<div class="flex flex-row items-center gap-2 shrink-0">
				{@render editorActions()}
			</div>
		</div>
		{@render draftBanner()}
		<div class="flex-1 min-h-0 overflow-auto p-4">
			{@render editorBody()}
		</div>
	</div>
{/if}
