<script lang="ts">
	import { VariableService, WorkspaceService } from '$lib/gen'
	import { createEventDispatcher, onDestroy, untrack } from 'svelte'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import OpenInSessionButton from './sessions/OpenInSessionButton.svelte'
	import {
		clearPageDrawerAnchor,
		handOffPageDrawer,
		pageDrawerSessionSource,
		setPageDrawerAnchor
	} from './sessions/pageDrawerSession'
	import { VARIABLES_PATH } from './sessions/previewPaths'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import { Save } from 'lucide-svelte'
	import VariableForm from './VariableForm.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { resource } from 'runed'
	import { useActingUser } from '$lib/actingUser.svelte'
	import { UserDraft, draftValuesEqual, type UserDraftHandle } from '$lib/userDraft.svelte'
	import LocalDraftBanner from './LocalDraftBanner.svelte'
	import DraftConflictAlert from './DraftConflictAlert.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { useDraftConflictSession } from '$lib/draftConflictSession.svelte'
	import { isEncryptedDraftValue } from '$lib/encryptedDraft'
	import { setLocalDraftHint } from '$lib/localDraftHints.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	const dispatch = createEventDispatcher()

	type VariableState = {
		path: string
		variable: { value: string; is_secret: boolean; description: string }
		labels: string[] | undefined
		wsSpecific: boolean
	}

	// The "current" workspace this editor defaults New/Edit actions to. Session
	// editors pass their acting workspace so secrets are created/updated there
	// rather than in the navigation workspace.
	let {
		workspace = undefined,
		inline = false,
		onClose = undefined,
		onSaved = undefined
	}: {
		workspace?: string
		/** Render in place, filling the parent, with no drawer or close button — for a host
		 * that gives the editor a whole pane. */
		inline?: boolean
		/** With `inline`, closes whatever hosts the editor; the header has a close button only when set. */
		onClose?: () => void
		/** Fires once a save lands, with the path the variable now lives at in `workspace` —
		 * not in the workspace-specific version selected, which can be another's. */
		onSaved?: (path: string) => void
	} = $props()
	// Sole ambient read in this file: the acting workspace is an input, and only its
	// default comes from the navigation store.
	let curWs = $derived(workspace ?? $operatingWorkspace)

	let editPath: string | undefined = $state(undefined)

	// Per-workspace handles are driven by `useMany`. We track the workspace
	// IDs (and their seeded defaults) in a parallel `$state` array; on every
	// mutation `useMany` reconciles, acquiring entries for new workspaces and
	// releasing them on component teardown. `states` indexes the resulting
	// handles by workspace ID for ergonomic lookup downstream.
	let workspaceSpecs = $state<Array<{ ws: string; defaultValue: VariableState }>>([])
	// Plain objects keyed by workspace id, so an id that is also an `Object.prototype` key
	// (`constructor`, …) reads as already present and the variable never loads. Such ids are
	// deliberately unsupported: too unlikely to be worth guarding every read.
	let initialStates: Record<string, VariableState> = $state({})
	let existedInitially: Record<string, boolean> = $state({})
	let extraPerms: Record<string, Record<string, boolean>> = $state({})
	let selected: string | undefined = $state(undefined)
	let pathError = $state('')
	const acting = useActingUser(() => selected)

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
	// `undefined` until the selected workspace's permissions and acting user have both
	// landed — a pending verdict is neither a grant nor the denial the read-only alert
	// announces, so the two must stay distinguishable.
	const can_write: boolean | undefined = $derived.by(() => {
		if (!selected || !edit) return true
		const perms = extraPerms[selected]
		if (!perms || !acting.resolved(selected)) return undefined
		return canWrite(editPath ?? '', perms, acting.in(selected))
	})
	const dirtyWorkspaces = $derived(
		Object.keys(states).filter((ws) => !draftValuesEqual(states[ws].draft, initialStates[ws]))
	)

	/** Scoped by `selected` so switching workspace-specific versions is a new session. Ended on top
	 *  of that by every entry point and by the teardown, since reopening the same variable reuses
	 *  this component. */
	const conflictSession = useDraftConflictSession(() => selected)
	function endEditingSession(): void {
		conflictSession.end()
	}
	onDestroy(endEditingSession)
	const resolvingConflict = $derived(conflictSession.busy)
	/** The server refused this tab's autosave because the row moved under it: another tab, or the
	 *  AI chat, which writes these drafts too. Nothing typed here reaches the server until the user
	 *  picks a version, and the unsaved-changes banner says the opposite — that the edits are held
	 *  as a draft — so without this they are told their work is safe while it is being dropped. */
	const draftConflict = $derived(
		edit && selected && editPath
			? UserDraftDbSyncer.getConflict({
					workspace: selected,
					itemKind: 'variable',
					path: editPath
				}).conflict
			: undefined
	)

	async function resolveDraftConflict(keepMine: boolean): Promise<void> {
		const ws = selected
		const p = editPath
		if (!ws || !p || resolvingConflict) return
		const query = { workspace: ws, itemKind: 'variable' as const, path: p }
		const token = conflictSession.start()
		const stillOurs = () => conflictSession.holds(token) && selected === ws
		try {
			if (keepMine) {
				// Settle the key first: an ordinary autosave still queued would displace the forced
				// write below, and being conditional it would be refused — so "Keep mine" would
				// finish without keeping anything and leave the alert standing.
				await UserDraftDbSyncer.quiesce(query)
				// A resolution belongs to the session that started it. One that outlives its editor
				// stops here rather than writing on: whatever replaced it — another session on the
				// same draft, or its own resolution — owns the key now, and the edit this one was
				// keeping is still parked for a later flush either way.
				if (!stillOurs()) return
				// A parked `null` is this tab's "no draft any more" — a discard, or an edit that
				// landed back on the deployed value. Keeping that means removing the row, not
				// writing the baseline back as a draft with no dirty banner to discard it through.
				const parked = UserDraftDbSyncer.peekPending(query)
				const mine = parked?.value === null ? null : $state.snapshot(states[ws]?.draft)
				// Forced, so it goes over the row that refused us, and its response reseeds
				// `last_sync` so the next ordinary save is conditional again.
				if (mine !== undefined) await UserDraftDbSyncer.overwrite({ ...query, value: mine })
				// Say so rather than leave the alert up with no explanation: a write displaced by
				// something typed meanwhile can still lose the race.
				if (UserDraftDbSyncer.getConflict(query).conflict) {
					sendUserToast('Could not keep your version — try again', true)
				}
				return
			}
			// Settle the key BEFORE reading, so what comes back is the version the server is left
			// holding: a write this tab started can still be in flight — a forced one it walked
			// away from included — and a response fetched past it describes a version about to be
			// replaced, which would then be seeded along with its already-stale `last_sync`.
			// Nothing is given up by waiting; `quiesce` only stops the pipeline.
			await UserDraftDbSyncer.quiesce(query)
			if (!stillOurs()) return
			// Read BEFORE giving anything up: until the server has answered, the refused payload is
			// still the only copy of this tab's edit, and the conflict is still true.
			const v = await VariableService.getVariable({
				workspace: ws,
				path: p,
				decryptSecret: false,
				getDraft: true
			})
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
			// Everything below writes shared editor state, so first make sure it is still this
			// variable's: the drawer stays closable while the read is out, and another variable
			// opened meanwhile would otherwise get this one's baseline — and with it this one's
			// path as its save target.
			if (!stillOurs()) return
			// Now, and not in `quiesce`: the refused payload belongs to the version being replaced,
			// but until this point it was still the only copy of the edit, and a resolution that
			// gave up before here has to leave it behind.
			UserDraftDbSyncer.dropPending(query)
			UserDraftDbSyncer.clearConflict(query)
			initialStates[ws] = structuredClone(deployedState)
			// Everything else the load path takes from this same response. The variable can have
			// been deleted, recreated, or had its permissions changed while the conflict stood,
			// and these decide create-vs-update and write access — so refreshing only what is
			// displayed would leave those deciding on the version the user just replaced.
			existedInitially[ws] = !(v as any).no_deployed
			extraPerms[ws] = v.extra_perms ?? {}
			UserDraftDbSyncer.recordRemoteSync(query, (v as any).draft_saved_at)
			UserDraft.seed(
				'variable',
				p,
				((v as any).draft as VariableState | undefined) ?? deployedState,
				{ workspace: ws }
			)
		} catch (e) {
			// Nothing was given up above, so the conflict stands and the edit is still here to
			// resolve again — which is the whole point of reading first.
			sendUserToast(`Could not load the other version: ${e}`, true)
		} finally {
			conflictSession.finish(token)
		}
	}

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
			return !perms || canWrite(editPath ?? '', perms, acting.in(ws))
		})
	)

	// Lazy-fetch the variable for the selected workspace when not already cached
	$effect(() => {
		const ws = selected
		const p = editPath
		if (!ws || !p) return
		if (ws in states) return
		untrack(() => {
			VariableService.getVariable({
				workspace: ws,
				path: p,
				decryptSecret: false,
				getDraft: true
			}).then((v) => {
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
				// A refused save leaves this tab's own version parked. `.draft` is the version
				// that refused it, so opening on that would quietly drop the edit the alert is
				// about and leave "Keep mine" offering to keep the other one.
				const conflictQuery = { workspace: ws, itemKind: 'variable' as const, path: p }
				const refused = UserDraftDbSyncer.getConflict(conflictQuery).conflict
					? UserDraftDbSyncer.peekPending(conflictQuery)
					: undefined
				// A parked `null` is this tab's "no draft any more", which on screen is the
				// deployed value.
				const refusedDraft = (refused?.value ?? undefined) as VariableState | undefined
				// Open with this tab's refused version if there is one, else the saved draft,
				// else the deployed.
				const s: VariableState = refused
					? (refusedDraft ?? deployedState)
					: (savedDraftState ?? deployedState)
				ensureHandle(ws, s)
				initialStates[ws] = structuredClone(deployedState)
				// Draft-only paths (`no_deployed`) have no row — saving must
				// CREATE, not update (update 404s).
				existedInitially[ws] = !(v as any).no_deployed
				extraPerms[ws] = v.extra_perms ?? {}
			})
		})
	})

	function reset() {
		// A new session starts here, so anything still running for the last one is spent.
		endEditingSession()
		// Clearing workspaceSpecs triggers useMany's reconcile to release
		// every acquired entry. The $derived `states` then collapses to {}.
		workspaceSpecs = []
		initialStates = {}
		existedInitially = {}
		extraPerms = {}
		pathError = ''
		acting.forgetFailures()
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
		if (handOffPageDrawer(VARIABLES_PATH, edit_path)) return
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
		const dirty = dirtyWorkspaces
		const savedPath = (curWs ? states[curWs]?.draft?.path : undefined) ?? editPath ?? ''
		try {
			for (const ws of dirty) {
				const s = states[ws].draft!
				const ini = initialStates[ws]
				if (existedInitially[ws]) {
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
				// The just-saved state is the new deployed baseline; reset the
				// handle to it via `discard` (not `remove` — blanking the cell to
				// `undefined` reads as dirty). The `value: null` POST also deletes
				// the server draft row so `is_draft` clears on refetch.
				initialStates[ws] = $state.snapshot(s) as VariableState
				existedInitially[ws] = true
				UserDraft.discard('variable', editPath ?? '', s, { workspace: ws })
				// Path now exists server-side — drop the autocomplete cache so
				// it shows up immediately instead of after the 60s TTL.
				invalidateWorkspacePaths(ws)
			}
			sendUserToast(edit ? `Updated variable in ${dirty.length} workspace(s)` : `Created variable`)
			dispatch('create')
			onSaved?.(savedPath)
			drawer?.closeDrawer()
		} catch (err) {
			sendUserToast(`Could not save variable: ${err.body}`, true)
		}
	}
</script>

{#if inline}
	{@render content()}
{:else}
	<Drawer
		bind:this={drawer}
		size="50rem"
		on:close={() => {
			endEditingSession()
			clearPageDrawerAnchor(VARIABLES_PATH)
		}}
	>
		{@render content()}
	</Drawer>
{/if}

{#snippet content()}
	<DrawerContent
		title={edit ? `Update variable at ${initialPath}` : 'Add a variable'}
		bannerReserved={edit}
		hideClose={inline && !onClose}
		fullScreen={!inline}
		on:close={() => {
			// Inline has no drawer to emit a close, so the session ends here instead.
			if (inline) {
				endEditingSession()
				onClose?.()
			} else {
				drawer?.closeDrawer()
			}
		}}
	>
		{#snippet banner()}
			{#if draftConflict}
				<DraftConflictAlert
					busy={resolvingConflict}
					onReload={() => void resolveDraftConflict(false)}
					onOverwrite={() => void resolveDraftConflict(true)}
				/>
			{/if}
			<LocalDraftBanner
				show={edit && selectedDirty}
				reserveSpace={edit}
				getDeployed={() => (selected ? initialStates[selected] : undefined)}
				getCurrent={() => current}
				onDiscard={() => {
					if (!selected) return
					UserDraft.discard('variable', editPath ?? '', initialStates[selected], {
						workspace: selected
					})
				}}
				disabled={!can_write}
			/>
		{/snippet}
		<div class="flex flex-col gap-8 pb-2">
			{#if can_write === false}
				<Alert type="warning" title="Only read access">
					You only have read access to this resource and cannot edit it
				</Alert>
			{/if}

			{#if otherDirty.length > 0}
				<Alert type="warning" title="Editing multiple workspaces">
					You are going to edit the value in: {otherDirty.join(', ')}
				</Alert>
			{/if}

			<!-- Held back until there is a verdict: rendering the form against a pending `can_write`
			would flash read-only controls at someone who can in fact write. -->
			{#if current && can_write !== undefined}
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
						can_write={can_write === true}
						{edit}
						onLoadSecret={loadSecret}
						workspace={selected}
						actingUser={acting.in(selected) ?? null}
					/>
				{/key}
			{/if}
		</div>
		{#snippet actions()}
			<OpenInSessionButton source={sessionSource} />
			{#if edit && curWs}
				<WsSpecificVersions kind="variable" workspaceId={curWs} {initialPath} bind:selected />
			{/if}
			<Button
				on:click={save}
				disabled={!anyDirty || !dirtyValid || !dirtyCanWrite || pathError != ''}
				startIcon={{ icon: Save }}
				variant="accent"
				size="sm"
			>
				{edit ? 'Update' : 'Save'}
			</Button>
		{/snippet}
	</DrawerContent>
{/snippet}
