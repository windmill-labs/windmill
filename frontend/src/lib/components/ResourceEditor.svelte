<script lang="ts">
	import type { Schema } from '$lib/common'
	import {
		GitSyncService,
		ResourceService,
		WorkspaceService,
		type Resource,
		type ResourceType
	} from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { createEventDispatcher, onDestroy, untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { clearJsonSchemaResourceCache } from './schema/jsonSchemaResource.svelte'
	import ResourceForm from './ResourceForm.svelte'
	import ReplaceGitCredential from './git_sync/ReplaceGitCredential.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { resource } from 'runed'
	import { useActingUser } from '$lib/actingUser.svelte'
	import { UserDraft, draftValuesEqual, type UserDraftHandle } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { useDraftConflictSession } from '$lib/draftConflictSession.svelte'
	import DraftConflictAlert from './DraftConflictAlert.svelte'
	import { setLocalDraftHint } from '$lib/localDraftHints.svelte'
	import { onUserInput } from '$lib/userDraftEditGate'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		canSave?: boolean
		resource_type?: string | undefined
		path?: string
		hidePath?: boolean
		onChange?: (args: { path: string; args: Record<string, any>; description: string }) => void
		defaultValues?: Record<string, any> | undefined
		/** Workspace this editor acts in — every call, permission check and cache key below
		 * derives from it. Optional for the packaged component; the navigation workspace is
		 * substituted once, at `effectiveWorkspace`, and nowhere else. */
		workspace?: string | undefined
		selected?: string | undefined
		/** Show the value as JSON rather than as the resource type's form. Bindable so a caller can
		 *  choose the view a given resource opens on, and the in-form toggle still works. */
		viewJsonSchema?: boolean
		/** Notifies the parent drawer whether a local draft for the selected
		 * workspace diverges from the deployed baseline, so it can show the
		 * "unsaved changes" banner below its header. */
		onDraftStateChange?: (hasDraft: boolean) => void
		/** Notifies the parent drawer of write-access for the selected workspace,
		 * so it can hide the banner's Discard button in read-only mode (matches
		 * the trigger editors' `disabled={!can_write}` wiring). */
		onCanWriteChange?: (canWrite: boolean) => void
		/** The drawer renders the conflict alert: inside this scrollable form it can land above the
		 *  viewport on a long resource, leaving the fixed banner claiming the edits are saved. */
		onDraftConflictChange?: (state: { conflicted: boolean; busy: boolean }) => void
	}

	let {
		canSave = $bindable(true),
		resource_type = $bindable(undefined),
		path = $bindable(''),
		hidePath = false,
		onChange,
		defaultValues = undefined,
		workspace = undefined,
		selected: selectedProp = $bindable(),
		viewJsonSchema = $bindable(),
		onDraftStateChange,
		onCanWriteChange,
		onDraftConflictChange
	}: Props = $props()

	type ResourceState = {
		path: string
		description: string
		args: Record<string, any>
		labels: string[] | undefined
		wsSpecific: boolean
	}

	const dispatch = createEventDispatcher()

	// Sole ambient read in this file: the acting workspace is an input, and only its
	// default comes from the navigation store.
	let effectiveWorkspace = $derived(workspace ?? $operatingWorkspace!)
	// Fallback to `effectiveWorkspace` insulates against reactify-style
	// parents that re-spread props without `selected` — otherwise it
	// transiently resets and the form below remounts on every keystroke.
	let selected = $derived(selectedProp ?? effectiveWorkspace)
	let initialPath = path

	// Per-workspace handles are driven by `useMany`. We track the workspace
	// IDs (and their seeded defaults) in a parallel `$state` array; on every
	// mutation `useMany` reconciles, acquiring entries for new workspaces and
	// releasing them on component teardown. `states` indexes the resulting
	// handles by workspace ID for ergonomic lookup downstream.
	let workspaceSpecs = $state<Array<{ ws: string; defaultValue: ResourceState }>>([])
	// Plain objects keyed by workspace id, so an id that is also an `Object.prototype` key
	// (`constructor`, …) reads as already present and the resource never loads. Such ids are
	// deliberately unsupported: too unlikely to be worth guarding every read.
	let initialStates: Record<string, ResourceState> = $state({})
	let existedInitially: Record<string, boolean> = $state({})
	let fetchedResources: Record<string, Resource> = $state({})
	const acting = useActingUser(() => selected)

	const handlesArray = UserDraft.useMany<ResourceState>(() =>
		workspaceSpecs.map((s) => ({
			itemKind: 'resource' as const,
			path: initialPath ?? '',
			workspace: s.ws,
			defaultValue: s.defaultValue,
			// Autosaves landing back on the deployed value become deletes;
			// `existedInitially` guards draft-only items from self-destructing.
			discardIf: (val) => !!existedInitially[s.ws] && draftValuesEqual(val, initialStates[s.ws])
		}))
	)
	const states = $derived.by(() => {
		const out: Record<string, UserDraftHandle<ResourceState>> = {}
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
	function ensureHandle(ws: string, defaultValue: ResourceState): void {
		if (workspaceSpecs.some((s) => s.ws === ws)) return
		workspaceSpecs.push({ ws, defaultValue })
	}

	// Gated per workspace until the user puts something into that workspace's
	// form (see `onUserInput`): the autosave stays suspended and the deployed
	// baseline absorbs whatever the form settles on. A workspace opened ON a
	// saved draft keeps its baseline — that divergence is the user's own.
	let userEdited: Record<string, boolean> = $state({})
	let openedOnDraft: Record<string, boolean> = $state({})
	const suspendedWorkspaces = new Set<string>()

	function setGated(ws: string, gated: boolean): void {
		if (!initialPath) return
		if (gated === suspendedWorkspaces.has(ws)) return
		if (gated) {
			UserDraft.stopSync('resource', initialPath, { workspace: ws })
			suspendedWorkspaces.add(ws)
		} else {
			UserDraft.restartSync('resource', initialPath, { workspace: ws })
			suspendedWorkspaces.delete(ws)
		}
	}

	// Nothing counts until this workspace's form is on screen, and while the
	// schema is still arriving a precursor alone does not: it would open the gate
	// just in time for the schema's materialized values to POST. `Path`, the
	// labels and the description render above that skeleton and stay editable
	// throughout, so a real value event still counts and keeps the edit.
	onUserInput((kind) => {
		if (!selected || !(selected in states)) return
		if (kind === 'precursor' && loadingSchema) return
		userEdited[selected] = true
	})

	$effect(() => {
		const wss = Object.keys(states)
		const edited = { ...userEdited }
		const onDraft = { ...openedOnDraft }
		untrack(() => {
			// A workspace opened on a saved draft is never suspended — there is no
			// phantom to prevent, and a write made while suspended is dropped for
			// good. Without `onDraft` here this effect re-suspends it the moment its
			// handle appears, undoing the decision made when it was opened.
			for (const ws of wss) setGated(ws, !edited[ws] && !onDraft[ws])
		})
	})

	// `stopSync` must be paired or the key stays unsynced for the session.
	onDestroy(() => {
		for (const ws of [...suspendedWorkspaces]) setGated(ws, false)
	})

	let isValid = $state(true)
	let jsonError = $state('')
	let perWsValid: Record<string, boolean> = $state({})

	const deployToResource = resource(
		() => selected,
		async (ws) =>
			ws ? (await WorkspaceService.getDeployTo({ workspace: ws })).deploy_to : undefined
	)
	const resourceTypeResource = resource(
		[() => resource_type, () => effectiveWorkspace],
		async ([rt, ws]) =>
			rt && ws ? ResourceService.getResourceType({ path: rt, workspace: ws }) : null
	)

	let deployTo = $derived(deployToResource.current)
	let resourceTypeInfo: ResourceType | undefined = $derived(
		resourceTypeResource.current ?? undefined
	)
	let resourceSchema: Schema | undefined = $derived.by(() => {
		const rt = resourceTypeResource.current
		if (!rt?.schema) return undefined
		const schema = rt.schema as Schema
		return {
			...schema,
			// A resource type may declare no properties at all — `dbt_profile` is a
			// `profiles.yml` block whose keys are its adapter's, not Windmill's — and
			// the form renders those as one JSON editor. `Object.keys(undefined)`
			// threw here, which left the editor on its loading skeleton forever.
			order: schema.order ?? Object.keys(schema.properties ?? {}).sort()
		}
	})
	let loadingSchema = $derived(resourceTypeResource.loading)

	let current = $derived(selected ? states[selected]?.draft : undefined)
	// The saved URL, not the draft's: a credential is bound to the repository it
	// is issued for, so binding one to an edit that has not landed yet would tie
	// it to something the resource does not point at.
	let deployedUrl = $derived(
		selected ? ((fetchedResources[selected]?.value as any)?.url as string | undefined) : undefined
	)
	// The deployed path, for the same reason as the deployed URL: the server
	// answers about what is stored, and an unsaved rename names nothing yet.
	let deployedPath = $derived(selected ? (initialStates[selected]?.path ?? initialPath) : undefined)
	// Asked of the server rather than read off the resource: the resource is
	// client-editable, exported and copied into forks, so nothing written on it
	// stays true. Re-asked when the saved URL moves, since that is a different
	// repository. The answer is for admins, who are the only ones who could act
	// on it, so nobody else asks.
	const credentialOrigin = resource(
		[
			() => selected,
			() => deployedPath,
			() => deployedUrl,
			() => resource_type,
			() => acting.in(selected)?.is_admin
		],
		async ([ws, path, _url, type, admin]) =>
			ws && path && type === 'git_repository' && admin
				? await GitSyncService.getCredentialOrigin({ workspace: ws, path }).catch(() => undefined)
				: undefined
	)
	// Only a credential this workspace holds is its to replace: a fork borrows
	// its ancestor's, and storing a replacement here would split it in two.
	let holdsCredential = $derived(credentialOrigin.current?.origin === 'held')
	// Only an unsaved *URL* blocks replacing the token, not any unsaved change:
	// opening the drawer materialises schema defaults (`folder: ""`), so a whole-
	// resource dirty check would disable it the moment the drawer opens.
	let urlDirty = $derived(!!deployedUrl && current?.args?.url !== deployedUrl)
	let resourceToEdit: Resource | undefined = $derived(
		selected ? fetchedResources[selected] : undefined
	)
	// `undefined` until both the resource and the acting user have landed — a pending verdict
	// is neither a grant nor the denial the read-only alert announces, so the two must stay
	// distinguishable.
	let can_write: boolean | undefined = $derived.by(() => {
		// A resource that does not exist yet has nobody's permissions on it.
		if (!initialPath || !selected) return true
		const r = fetchedResources[selected]
		if (!r || !acting.resolved(selected)) return undefined
		return canWrite(current?.path ?? initialPath, r.extra_perms ?? {}, acting.in(selected))
	})

	const dirtyWorkspaces = $derived(
		Object.keys(states).filter((ws) => !draftValuesEqual(states[ws].draft, initialStates[ws]))
	)
	const anyDirty = $derived(dirtyWorkspaces.length > 0)

	/** Scoped by `selected` so switching workspace-specific versions is a new session. Ending is
	 *  exported on top of that because the drawer, not this component, knows when its own session
	 *  is over: this component outlives it. */
	const conflictSession = useDraftConflictSession(() => selected)
	export function endEditingSession(): void {
		conflictSession.end()
	}
	onDestroy(endEditingSession)
	const resolvingConflict = $derived(conflictSession.busy)
	/** The server refused this tab's autosave because the row moved under it: another tab, or the
	 *  AI chat, which writes these drafts too. Nothing typed here reaches the server until the user
	 *  picks a version, and the unsaved-changes banner says the opposite — that the edits are held
	 *  as a draft — so without this they are told their work is safe while it is being dropped. */
	const draftConflict = $derived(
		selected && initialPath
			? UserDraftDbSyncer.getConflict({
					workspace: selected,
					itemKind: 'resource',
					path: initialPath
				}).conflict
			: undefined
	)

	async function resolveDraftConflict(keepMine: boolean): Promise<void> {
		const ws = selected
		const p = initialPath
		if (!ws || !p || resolvingConflict) return
		const query = { workspace: ws, itemKind: 'resource' as const, path: p }
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
			const r = await ResourceService.getResource({ workspace: ws, path: p, getDraft: true })
			const deployedState: ResourceState = {
				path: r.path,
				args: (r.value ?? {}) as Record<string, any>,
				description: r.description ?? '',
				labels: r.labels ?? undefined,
				wsSpecific: r.ws_specific ?? false
			}
			// Everything below writes shared editor state, so first make sure it is still this
			// resource's: the drawer stays closable while the read is out, and another resource
			// opened meanwhile would otherwise get this one's baseline — and with it this one's
			// path as its save target.
			if (!stillOurs()) return
			// Now, and not in `quiesce`: the refused payload belongs to the version being replaced,
			// but until this point it was still the only copy of the edit, and a resolution that
			// gave up before here has to leave it behind.
			UserDraftDbSyncer.dropPending(query)
			UserDraftDbSyncer.clearConflict(query)
			initialStates[ws] = structuredClone(deployedState)
			// Everything else the load path takes from this same response. The item can have been
			// deleted, recreated under another type, or had its permissions changed while the
			// conflict stood, and the fields below decide create-vs-update, the schema and write
			// access — so refreshing only what is displayed would leave those deciding on the
			// version the user just replaced.
			fetchedResources[ws] = r
			existedInitially[ws] = !(r as any).no_deployed
			if (ws === effectiveWorkspace) resource_type = r.resource_type
			UserDraftDbSyncer.recordRemoteSync(query, (r as any).draft_saved_at)
			const loadedDraft = (r as any).draft as ResourceState | undefined
			// Loading a draft makes this workspace one that opened with a draft, whatever it opened
			// with before — a refused deletion opens gated, and gated the settling absorber would
			// fold the version just loaded into the deployed baseline, leaving it silently clean
			// with Save disabled. Ungate here rather than leaving it to the effect, so no write
			// between the two is absorbed.
			openedOnDraft[ws] = !!loadedDraft
			if (loadedDraft) {
				setGated(ws, false)
			} else {
				// Accepting "there is no draft" has to shut the gate, the way discarding one does.
				// The edit that raised the conflict set `userEdited`, and left open, the form's
				// settling writes — schema defaults materializing over the loaded value — read as
				// the user's and recreate the draft just accepted away.
				userEdited[ws] = false
				setGated(ws, true)
			}
			UserDraft.seed('resource', p, loadedDraft ?? deployedState, { workspace: ws })
		} catch (e) {
			// Nothing was given up above, so the conflict stands and the edit is still here to
			// resolve again — which is the whole point of reading first.
			sendUserToast(`Could not load the other version: ${e}`, true)
		} finally {
			conflictSession.finish(token)
		}
	}

	// The syncer owns the list-page `*` hint; the editor only CLEARS it when a
	// workspace is at the deployed baseline (so a draft discarded elsewhere
	// vanishes on reopen). Never SET here. See VariableEditor for the full note.
	$effect(() => {
		const p = initialPath
		const loadedWs = Object.keys(states)
		const dirty = dirtyWorkspaces
		untrack(() => {
			if (!p) return
			for (const ws of loadedWs) {
				if (!dirty.includes(ws)) setLocalDraftHint(ws, 'resource', p, false)
			}
		})
	})
	// Banner is scoped to the selected workspace — the diff/discard only
	// operate on it, so showing it for an unrelated dirty workspace would be
	// misleading. The cross-workspace `otherDirty` alert below still covers
	// that case.
	const selectedDirty = $derived(!!selected && dirtyWorkspaces.includes(selected))
	const otherDirty = $derived(
		dirtyWorkspaces.length == 1
			? dirtyWorkspaces.filter((ws) => ws !== effectiveWorkspace)
			: dirtyWorkspaces
	)
	const dirtyValid = $derived(dirtyWorkspaces.every((ws) => perWsValid[ws] !== false))
	const dirtyCanWrite = $derived(
		dirtyWorkspaces.every((ws) => {
			const r = fetchedResources[ws]
			return (
				!r || canWrite(states[ws]?.draft?.path ?? initialPath, r.extra_perms ?? {}, acting.in(ws))
			)
		})
	)

	// New-resource bootstrap: seed empty state per workspace (edit mode
	// is seeded by the lazy-fetch effect below).
	$effect(() => {
		const ws = selected
		if (!ws) return
		if (initialPath) return
		if (ws in initialStates) return
		untrack(() => {
			const s: ResourceState = {
				path: '',
				description: '',
				args: (defaultValues && Object.keys(defaultValues).length > 0 ? defaultValues : {}) as any,
				labels: undefined,
				wsSpecific: false
			}
			ensureHandle(ws, s)
			initialStates[ws] = structuredClone(s)
			existedInitially[ws] = false
		})
	})

	// Lazy-fetch the resource for the selected workspace when not already cached
	$effect(() => {
		const ws = selected
		if (!ws || !initialPath) return
		if (ws in states) return
		untrack(() => {
			ResourceService.getResource({ workspace: ws, path: initialPath, getDraft: true }).then(
				(r) => {
					// `.draft` already holds the editor's `ResourceState` shape.
					const savedDraftState = (r as any).draft as ResourceState | undefined
					fetchedResources[ws] = r
					// Deployed baseline as the dirty-check reference, so the banner
					// compares draft-vs-deployed and fires immediately when a draft exists.
					const deployedState: ResourceState = {
						path: r.path,
						description: r.description ?? '',
						args: (r.value ?? {}) as any,
						labels: r.labels ?? undefined,
						wsSpecific: r.ws_specific ?? false
					}
					// A refused save leaves this tab's own version parked. `.draft` is the
					// version that refused it, so opening on that would quietly drop the edit
					// the alert is about and leave "Keep mine" offering to keep the other one.
					const conflictQuery = {
						workspace: ws,
						itemKind: 'resource' as const,
						path: initialPath
					}
					const refused = UserDraftDbSyncer.getConflict(conflictQuery).conflict
						? UserDraftDbSyncer.peekPending(conflictQuery)
						: undefined
					// A parked `null` is this tab's "no draft any more", which on screen is the
					// deployed value — so only a payload with content counts as a local draft.
					const refusedDraft = (refused?.value ?? undefined) as ResourceState | undefined
					const hasLocalDraft = refused ? refused.value !== null : !!savedDraftState
					// Open with this tab's refused version if there is one, else the saved
					// draft, else the deployed.
					const s: ResourceState = refused
						? (refusedDraft ?? deployedState)
						: (savedDraftState ?? deployedState)
					openedOnDraft[ws] = hasLocalDraft
					// Gate BEFORE the handle is acquired: `stopSync` queues on a
					// not-yet-live entry, and the form can settle before the effect
					// above gets a chance to run. Only worth doing when no draft exists
					// yet — where one does, there is no phantom to prevent and
					// suspending could only drop a write.
					if (!hasLocalDraft) setGated(ws, true)
					ensureHandle(ws, s)
					initialStates[ws] = structuredClone(deployedState)
					// Draft-only paths (`no_deployed`) have no row — saving must
					// CREATE, not update (update 404s).
					existedInitially[ws] = !(r as any).no_deployed
					// Keep resource_type in sync for the base workspace (controls the schema)
					if (ws === effectiveWorkspace) {
						resource_type = r.resource_type
					}
				}
			)
		})
	})

	/** The schema can only ever write `args`. `path`, `labels`, `description` and
	 * `wsSpecific` are beyond its reach, so a difference in one of those is the
	 * user's — whatever event did or didn't reach the gate. Removing a label runs
	 * a click handler and emits nothing native, and would otherwise be absorbed. */
	function differsOutsideArgs(a: ResourceState, b: ResourceState | undefined): boolean {
		return !!b && !draftValuesEqual({ ...a, args: null }, { ...b, args: null })
	}

	// Absorb the form's settling writes into the deployed baseline while the
	// selected workspace is gated, so they show up neither as the "unsaved
	// changes" banner nor, once `discardIf` reads the baseline, as a draft.
	// Only the selected workspace has a form rendered against it.
	$effect(() => {
		const ws = selected
		if (!ws || !initialPath) return
		if (userEdited[ws] || openedOnDraft[ws]) return
		// `$state.snapshot` deep-reads, so nested `args` mutations re-run this.
		const settled = states[ws]?.draft
			? ($state.snapshot(states[ws].draft) as ResourceState)
			: undefined
		untrack(() => {
			if (!settled) return
			if (differsOutsideArgs(settled, initialStates[ws])) {
				// An edit, not settling. This runs AFTER the write landed, and a write
				// made while suspended is swallowed for good (the mirror advances its
				// baseline either way), so un-suspend and push the value here rather
				// than leaving it to whichever effect happens to run next.
				userEdited[ws] = true
				setGated(ws, false)
				void UserDraftDbSyncer.save({
					workspace: ws,
					itemKind: 'resource',
					path: initialPath,
					value: settled
				})
				return
			}
			if (!draftValuesEqual(settled, initialStates[ws])) initialStates[ws] = settled
		})
	})

	// Keep current.path bound to the outer `path` prop for consumers
	$effect(() => {
		if (current) path = current.path
	})

	$effect(() => {
		if (selected !== undefined) {
			perWsValid[selected] = isValid && jsonError == ''
		}
	})

	$effect(() => {
		canSave = anyDirty && dirtyValid && dirtyCanWrite
	})

	// Drive the parent drawer's "unsaved changes" banner. The drawer chrome
	// (header + banner slot) lives in ResourceEditorDrawer, above this
	// lazily-imported content, so the state is lifted up via these accessors.
	$effect(() => {
		onDraftStateChange?.(!!initialPath && selectedDirty)
	})
	$effect(() => {
		onCanWriteChange?.(can_write === true)
	})
	$effect(() => {
		onDraftConflictChange?.({ conflicted: !!draftConflict, busy: resolvingConflict })
	})

	export function resolveDraftConflictFromBanner(keepMine: boolean): void {
		void resolveDraftConflict(keepMine)
	}

	export function localDraftDeployed(): ResourceState | undefined {
		return selected ? initialStates[selected] : undefined
	}
	export function localDraftCurrent(): ResourceState | undefined {
		return current
	}
	export function discardLocalDraft(): void {
		if (!selected) return
		// Back to the deployed value with nothing of the user's left in it, so
		// the gate closes again — otherwise the form settles on the schema's
		// values a second time and the discarded draft comes straight back.
		// `discard` POSTs the delete itself, so suspending first is safe.
		openedOnDraft[selected] = false
		userEdited[selected] = false
		setGated(selected, true)
		UserDraft.discard('resource', initialPath ?? '', initialStates[selected], {
			workspace: selected
		})
	}

	$effect(() => {
		if (current)
			// $state.snapshot deep-reads (so the effect re-runs on nested
			// args mutations) and returns a plain object (React consumers
			// can't diff a $state proxy by reference or JSON.stringify).
			onChange?.({
				path: current.path,
				args: $state.snapshot(current.args) as Record<string, any>,
				description: current.description
			})
	})

	/** Sole writer of `current.path` — an arg still holding `$var:<the path being
	 * replaced>` is the resource's own linked secret, which the backend renames
	 * along with the resource, so the reference moves with it. An arg pointing at
	 * any other variable was set by the user and is left alone. */
	function setPath(npath: string): void {
		if (!current) return
		const prev = current.path
		// `args` is whatever the raw JSON editor parsed — `null` included.
		for (const [k, v] of Object.entries(current.args ?? {})) {
			if (v === `$var:${prev}`) current.args[k] = `$var:${npath}`
		}
		current.path = npath
	}

	/** The path the resource has in `ws`: after a save, the one it was saved under. Each
	 * workspace-specific version keeps its own, so the selected one says nothing about `ws`. */
	export function pathIn(ws: string): string | undefined {
		return initialStates[ws]?.path
	}

	/** Whether the write landed. It toasts its own failure, so most callers ignore this;
	 * one that follows the save with bookkeeping of its own has to know not to. */
	export async function save(): Promise<boolean> {
		const dirty = dirtyWorkspaces
		try {
			for (const ws of dirty) {
				const s = states[ws].draft!
				const ini = initialStates[ws]
				if (existedInitially[ws]) {
					await ResourceService.updateResource({
						workspace: ws,
						path: ini.path,
						requestBody: {
							path: s.path,
							value: s.args,
							description: s.description,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
					const fetched = fetchedResources[ws]
					if (fetched?.resource_type === 'json_schema') {
						clearJsonSchemaResourceCache(ini.path, ws)
					}
				} else {
					await ResourceService.createResource({
						workspace: ws,
						requestBody: {
							path: s.path,
							value: s.args,
							description: s.description,
							resource_type: resource_type!,
							labels: s.labels,
							ws_specific: s.wsSpecific
						}
					})
				}
				// Reset the handle to the new deployed baseline via `discard`, not
				// `remove`. See VariableEditor for the full rationale.
				initialStates[ws] = $state.snapshot(s) as ResourceState
				existedInitially[ws] = true
				UserDraft.discard('resource', initialPath ?? '', s, { workspace: ws })
				// Path now exists server-side — drop the autocomplete cache so
				// it shows up immediately instead of after the 60s TTL.
				invalidateWorkspacePaths(ws)
			}
			sendUserToast(
				dirty.length > 1 ? `Saved resource in ${dirty.length} workspaces` : `Saved resource`
			)
			dispatch('refresh', current?.path ?? path)
			return true
		} catch (err) {
			sendUserToast(`Could not save resource: ${err.body ?? err.message}`, true)
			return false
		}
	}
</script>

<div>
	<div class="flex flex-col gap-6 pb-2">
		<!-- Only when nobody above is showing it. A host that takes `onDraftConflictChange` puts it
		     in its own fixed banner, where a long form cannot scroll it out of view; one that embeds
		     this editor directly — the AI chat's MCP section, and the SDK surface — would otherwise
		     get no warning at all, which is the very thing this alert exists to prevent. -->
		{#if draftConflict && !onDraftConflictChange}
			<DraftConflictAlert
				busy={resolvingConflict}
				onReload={() => void resolveDraftConflict(false)}
				onOverwrite={() => void resolveDraftConflict(true)}
			/>
		{/if}

		{#if otherDirty.length > 0}
			<Alert type="warning" title="Editing multiple workspaces">
				You are going to edit the value in: {otherDirty.join(', ')}
			</Alert>
		{/if}

		{#if holdsCredential && selected}
			<Alert type="info" title="Windmill holds this repository's access token">
				<div class="flex flex-col items-start gap-2">
					<div>
						The URL carries no credential. Windmill stores the token and renews it before it
						expires.
						{#if urlDirty}
							Save your URL change to replace the token.
						{/if}
					</div>
					<ReplaceGitCredential
						workspace={selected}
						repoUrl={deployedUrl ?? ''}
						disabled={urlDirty || !deployedUrl}
					/>
				</div>
			</Alert>
		{/if}

		<!-- Held back until there is a verdict: rendering the form against a pending `can_write`
			would flash read-only controls at someone who can in fact write. -->
		{#if current && can_write !== undefined}
			{#key current}
				<ResourceForm
					bind:path={() => current!.path, setPath}
					bind:labels={current.labels}
					bind:description={current.description}
					bind:args={current.args}
					bind:wsSpecific={current.wsSpecific}
					bind:isValid
					bind:viewJsonSchema={() => viewJsonSchema ?? false, (v) => (viewJsonSchema = v)}
					bind:jsonError
					{initialPath}
					{hidePath}
					{deployTo}
					{can_write}
					{resource_type}
					{resourceTypeInfo}
					{resourceSchema}
					{loadingSchema}
					{resourceToEdit}
					onLoadResourceType={() => resourceTypeResource.refetch()}
					workspace={selected}
					actingUser={acting.in(selected) ?? null}
				/>
			{/key}
		{/if}
	</div>
</div>
