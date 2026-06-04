<script lang="ts">
	import { ScriptService, type NewScript, type Script } from '$lib/gen'

	import { initialArgsStore, userStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { cleanValueProperties, orderedJsonStringify } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import LocalDraftStaleModal from '$lib/components/common/confirmationModal/LocalDraftStaleModal.svelte'
	import OtherUsersDraftsModal from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import {
		UserDraft,
		checkStaleness,
		type UserDraftMeta,
		type UserDraftHandle
	} from '$lib/userDraft.svelte'
	import { notifyDraftLoaded, notifyRestoredFromLocal } from '$lib/userDraftToast'

	type EditableScript = NewScript & { draft_triggers?: Trigger[] }

	// `initialArgs` is intentionally captured once at mount — it's the
	// session's initial argument set, not per-script.
	let initialArgs = get(initialArgsStore) ?? {}
	if (get(initialArgsStore)) $initialArgsStore = undefined

	// Derived so client-side nav (breadcrumb) re-reads the URL, not mount-time values.
	let topHash = $derived(page.url.searchParams.get('topHash') ?? undefined)

	let hash = $derived(page.url.searchParams.get('hash') ?? undefined)

	// When viewing a specific historical hash we don't want to load or write a
	// local draft — that view is read-only relative to drafts.
	let draftPath = $derived(hash ? '' : (page.params.path ?? ''))

	// `useMany` keyed off the reactive `draftPath` re-keys the handle on nav;
	// `scriptHandle` proxies the current handle so `bind:script` stays a fixed lvalue.
	const scriptHandles = UserDraft.useMany<EditableScript>(() => [
		{ itemKind: 'script', path: draftPath }
	])
	const scriptHandle: UserDraftHandle<EditableScript> = {
		get draft() {
			return scriptHandles[0]?.draft
		},
		set draft(value) {
			const handle = scriptHandles[0]
			if (handle) handle.draft = value
		},
		get meta() {
			return scriptHandles[0]?.meta ?? {}
		},
		setDraftAndMeta(value, meta) {
			scriptHandles[0]?.setDraftAndMeta(value, meta)
		},
		setMeta(meta, opts) {
			scriptHandles[0]?.setMeta(meta, opts)
		}
	}

	$effect(() => {
		if (hash || !$workspaceStore) return
		const workspace = $workspaceStore
		UserDraft.setLiveEditorDraft({
			workspace,
			itemKind: 'script',
			storagePath: draftPath,
			effectivePath: scriptHandle.draft?.path ?? draftPath
		})
		return () => UserDraft.clearLiveEditorDraft('script', { workspace, storagePath: draftPath })
	})

	// Seed from the URL so ScriptBuilder mounts with a populated `initialPath`
	// even when `scriptHandle.draft` is already defined synchronously from a
	// local autosave. An empty initialPath flips ScriptBuilder's
	// `metadataOpen` heuristic (intended for /scripts/add) into "true" and
	// pops the settings drawer open on /edit.
	let initialPath: string = $state(hash ? '' : (page.params.path ?? ''))

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	let savedScript: Script | NewScript | undefined = $state(undefined)
	let fullyLoaded = $state(false)

	// Remounts ScriptBuilder on nav: false while a reload runs, true once data is
	// ready. A synchronous `{#key}` swap instead races Monaco's init against the
	// torn-down container (mirrors how the raw-app editor clears `files`).
	let renderEditor = $state(false)

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	// Local-draft staleness modal: opened when the remote (deployed or DB
	// draft) has moved on since the user's autosave was created.
	let staleModalOpen = $state(false)
	let staleModalCause = $state<'draft' | 'version'>('version')
	let pendingBaseline: { baseline: EditableScript; revs: UserDraftMeta } | undefined = undefined

	function applyBaseline(baseline: EditableScript): void {
		initialPath = baseline.path
		scriptBuilder?.setDraftTriggers(baseline.draft_triggers)
		scriptBuilder?.setCode(baseline.content)
		if (baseline['primary_schedule']) {
			savedPrimarySchedule = baseline['primary_schedule']
			scriptBuilder?.setPrimarySchedule(savedPrimarySchedule)
		}
	}

	function onStaleLoadLatest(): void {
		if (!pendingBaseline) {
			staleModalOpen = false
			return
		}
		const { baseline, revs } = pendingBaseline
		UserDraft.remove('script', draftPath)
		scriptHandle.setDraftAndMeta(baseline, revs)
		applyBaseline(baseline)
		pendingBaseline = undefined
		staleModalOpen = false
	}

	function onStaleKeepDraft(): void {
		if (pendingBaseline) {
			scriptHandle.setMeta(pendingBaseline.revs, { force: true })
		}
		pendingBaseline = undefined
		staleModalOpen = false
	}

	/** Increments per `loadScript` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadScriptToken = 0
	async function loadScript(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadScriptToken
		fullyLoaded = false
		// `?new_draft=true` (set by `/scripts/add`'s redirect) means we
		// landed on a fresh `u/{user}/draft_{uuid}` path that's never
		// been saved. Skip the backend fetch (it would 404), seed an
		// empty `NewScript` whose `path` is intentionally empty: the
		// `Path` widget's `initPath` calls `reset()` when both `path`
		// and `initialPath` are empty, which is what generates the
		// friendly `<random_adj>_<kind>` name. Anything non-empty (even
		// `u/{user}/`) is parsed verbatim and the friendly seed never
		// fires. `initialPath = ''` also makes ScriptBuilder open the
		// metadata drawer on mount. Strip the single-use flag last.
		if (page.url.searchParams.get('new_draft') === 'true') {
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			const empty: EditableScript = {
				path: '',
				summary: '',
				description: '',
				content: '',
				language: 'bun',
				schema: {}
			} as unknown as EditableScript
			initialPath = ''
			savedScript = structuredClone(empty)
			scriptHandle.setDraftAndMeta(empty, {})
			fullyLoaded = true
			renderEditor = true
			return
		}
		if (hash) {
			const scriptByHash = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash
			})
			if (tok !== loadScriptToken) return
			savedScript = structuredClone($state.snapshot(scriptByHash))
			scriptHandle.draft = { ...scriptByHash, parent_hash: hash, lock: undefined }
		} else {
			const backendScript = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: page.params.path ?? '',
				getDraft
			})
			if (tok !== loadScriptToken) return
			if (backendScript.is_draft) {
				notifyDraftLoaded({
					workspace: $workspaceStore!,
					itemKind: 'script',
					path: page.params.path ?? '',
					draftOnly: backendScript.no_deployed,
					onResetToDeployed: async () => {
						// Drop the in-memory draft and refetch *without* the
						// draft overlay — we don't trust the eventual delete
						// to have landed, so we read deployed directly.
						scriptHandle.setDraftAndMeta(undefined, {})
						await loadScript({ getDraft: false })
					}
				})
			}
			// `backendScript` is the deployed payload; the user's saved
			// draft (if any) sits in `.draft`. Layer the draft over the
			// deployed at the field level — the draft contributes editor
			// state (content, summary, …) and the deployed contributes
			// metadata the draft never carries (hash, version markers).
			// Same effective shape the backend used to deep-merge for us;
			// kept frontend-side now so the draft's editor shape can
			// diverge from the wire shape without confusing the server.
			const { draft: draftFromBackend, ...deployedScript } = backendScript as any
			const effectiveScript: EditableScript = draftFromBackend
				? { ...deployedScript, ...draftFromBackend }
				: (deployedScript as EditableScript)
			savedScript = structuredClone($state.snapshot(effectiveScript))

			const localDraft = scriptHandle.draft
			const previousMeta = scriptHandle.meta
			const newRevs: UserDraftMeta = {
				remoteRev: backendScript.hash
			}

			// Compute the fully-baked initial value once so the assignment
			// below is a single write — otherwise post-load mutations like
			// `parent_hash = ...` would count as a second write under
			// useLocalStorageValue's saveInitialValue=false contract and get
			// persisted before the user has touched anything.
			const bakedBaseline: EditableScript = {
				...effectiveScript,
				parent_hash: topHash ?? backendScript.hash
			}

			if (localDraft != undefined) {
				const referenceClean = cleanValueProperties(effectiveScript)
				const localClean = cleanValueProperties(localDraft)
				if (orderedJsonStringify(referenceClean) === orderedJsonStringify(localClean)) {
					// Local matches the saved version — silently drop it and use the saved one.
					UserDraft.remove('script', draftPath)
					scriptHandle.setDraftAndMeta(bakedBaseline, newRevs)
				} else {
					const cause = checkStaleness(previousMeta, newRevs.remoteRev, newRevs.remoteDraftRev)
					if (cause) {
						// Remote moved on since the local autosave was written —
						// surface the choice via modal. The local draft stays on
						// screen until the user picks.
						pendingBaseline = { baseline: bakedBaseline, revs: newRevs }
						staleModalCause = cause
						staleModalOpen = true
					} else {
						if (previousMeta.remoteRev === undefined && previousMeta.remoteDraftRev === undefined) {
							// Legacy entry (no meta recorded) — backfill so future
							// loads can detect staleness even if the user doesn't edit.
							scriptHandle.setMeta(newRevs, { force: true })
						}
						const scriptPath = bakedBaseline.path
						notifyRestoredFromLocal(false, true, {
							onResetToSavedDraft: () => {
								UserDraft.remove('script', draftPath)
								scriptHandle.setDraftAndMeta(bakedBaseline, newRevs)
								applyBaseline(bakedBaseline)
							},
							onResetToDeployed: async () => {
								UserDraft.remove('script', draftPath)
								// UserDraft.remove only clears localStorage. The entry's
								// in-memory state is kept alive by this route's handle, so
								// loadScript would re-read the stale autosave and the toast
								// would fire again. Drop the in-memory state first.
								scriptHandle.setDraftAndMeta(undefined, {})
								goto(`/scripts/edit/${scriptPath}`)
								loadScript()
							}
						})
					}
				}
			} else {
				scriptHandle.setDraftAndMeta(bakedBaseline, newRevs)
			}
		}

		if (scriptHandle.draft) {
			initialPath = scriptHandle.draft.path
			scriptBuilder?.setDraftTriggers(scriptHandle.draft.draft_triggers)
			scriptBuilder?.setCode(scriptHandle.draft.content)
		}
		fullyLoaded = true
		renderEditor = true
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one script editor
		// to another (e.g. via the workspace picker) reloads the new script.
		page.params.path
		if ($workspaceStore) {
			untrack(() => {
				renderEditor = false // remount the builder for the navigated-to script
				loadScript().catch((e: any) => {
					// A failed load must NOT leave renderEditor stuck false — otherwise
					// the editor pane disappears and never remounts. Surface the error
					// and remount so the user isn't stranded on a blank pane.
					console.error('Failed to load script', e)
					sendUserToast(`Failed to load script: ${e?.body ?? e?.message ?? e}`, true)
					renderEditor = true
				})
			})
		}
	})

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreDeployed() {
		if (!savedScript) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.remove('script', draftPath)
		scriptHandle.setDraftAndMeta(undefined, {})
		goto(`/scripts/edit/${savedScript.path}`)
		loadScript()
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
<LocalDraftStaleModal
	open={staleModalOpen}
	cause={staleModalCause}
	onLoadLatest={onStaleLoadLatest}
	onKeepDraft={onStaleKeepDraft}
/>
{#if !hash && $workspaceStore && page.params.path}
	<OtherUsersDraftsModal
		workspace={$workspaceStore}
		itemKind="script"
		path={page.params.path}
		currentValue={scriptHandle.draft}
		currentUserEmail={$userStore?.email}
		{diffDrawer}
		userHasLocalDraft={UserDraft.has('script', draftPath)}
		onFork={(otherValue) => {
			UserDraft.save('script', draftPath, otherValue, { workspace: $workspaceStore })
			diffDrawer?.closeDrawer()
		}}
	/>
{/if}
{#if scriptHandle.draft && renderEditor}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		userDraftPath={draftPath}
		bind:script={scriptHandle.draft}
		{fullyLoaded}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
		{savedPrimarySchedule}
		searchParams={page.url.searchParams}
		onDeploy={(e) => {
			// "Deploy & Stay here" / lib: stay on the editor (just confirm).
			if (e.stay) {
				sendUserToast('Deployed')
				return
			}
			UserDraft.remove('script', draftPath)
			if ($workspaceStore) invalidate($workspaceStore, 'script')
			goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
		}}
		onSeeDetails={(e) => {
			goto(`/scripts/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onNavigate={(item) => goto(editPathFor(item))}
	/>
{/if}
