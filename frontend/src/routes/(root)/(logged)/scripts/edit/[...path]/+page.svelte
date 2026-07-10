<script lang="ts">
	import { ScriptService, type NewScript, type Script, type ScriptLang } from '$lib/gen'

	import { initialArgsStore, workspaceStore } from '$lib/stores'
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import { editPathFor, invalidate } from '$lib/components/workspacePicker'
	import { decodeState, emptySchema, emptyString } from '$lib/utils'
	import { replaceScriptPlaceholderWithItsValues } from '$lib/hub'
	import { isWorkflowAsCode } from '$lib/components/graph/wacToFlow'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { type OtherDraftUser } from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import DraftEditorModals from '$lib/components/common/confirmationModal/DraftEditorModals.svelte'
	import type { ScheduleTrigger } from '$lib/components/triggers'
	import type { Trigger } from '$lib/components/triggers/utils'
	import { get } from 'svelte/store'
	import { onDestroy, untrack } from 'svelte'
	import { stripNewDraftFlagOnSave } from '$lib/newDraftFlag'
	import { page } from '$app/state'
	import { UserDraft, draftValuesEqual } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { discardDraftAfterDeploy, runResetToDeployed } from '$lib/userDraftToast'
	import { usePageDraftSync } from '$lib/components/usePageDraftSync.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'
	import { OtherUserDraftLoad } from '$lib/components/otherUserDraftLoad.svelte'

	type EditableScript = NewScript & { draft_triggers?: Trigger[] }

	// Captured once at mount: a session-wide arg set, not per-script. URL form
	// (`?initial_args=`, e.g. run page's "Fork") wins over the store form.
	const urlArgs = page.url.searchParams.get('initial_args')
	let initialArgs = urlArgs ? decodeState(urlArgs) : (get(initialArgsStore) ?? {})
	if (get(initialArgsStore)) $initialArgsStore = undefined

	/** Some pages base64-JSON-encode a NewScript payload into the `/scripts/add`
	 * link's URL hash (preserved by the redirect); it wins over templates, hubs
	 * and imports. Not `decodeState`: that toasts on parse failure, which would
	 * fire for non-payload hashes (e.g. a route anchor). */
	function decodeUrlScript(): Partial<EditableScript> | undefined {
		const fragment = page.url.hash.startsWith('#') ? page.url.hash.slice(1) : ''
		if (!fragment) return undefined
		try {
			const decoded = JSON.parse(decodeURIComponent(atob(fragment)))
			if (decoded && typeof decoded === 'object') return decoded as Partial<EditableScript>
		} catch {
			// Hash isn't a valid encoded script — ignore.
		}
		return undefined
	}

	// Template-seeding state set in the `new_draft` branch: `builderTemplate`
	// selects WAC mode, `lockedLanguage` pins the picker for hub/template forks,
	// `pathChosen` stops ScriptBuilder's summary→path auto-slug for explicit paths.
	let builderTemplate: 'script' | 'wac_python' | 'wac_typescript' = $state('script')
	let lockedLanguage = $state(false)
	let pathChosen = $state(false)

	// Derived so client-side nav (breadcrumb) re-reads the URL, not mount-time values.
	let topHash = $derived(page.url.searchParams.get('topHash') ?? undefined)

	let hash = $derived(page.url.searchParams.get('hash') ?? undefined)

	// When viewing a specific historical hash we don't want to load or write a
	// local draft — that view is read-only relative to drafts.
	let draftPath = $derived(hash ? '' : (page.params.path ?? ''))

	// Page-level draft orchestration: autosave handle (re-keyed on nav via
	// `draftPath`), live-editor-draft registry, `recordRemoteSync`, removal.
	// `draftSync.draft` stays a stable lvalue for `bind:script`.
	/** Deployed script this load (with `parent_hash` grafted to match the
	 * unedited draft seed), the baseline the autosave `discardIf` compares
	 * against. `undefined` for draft-only paths so they never self-destruct. */
	let deployedBaseline = $state<EditableScript | undefined>(undefined)

	const draftSync = usePageDraftSync<EditableScript>({
		itemKind: 'script',
		path: () => draftPath,
		workspace: () => $workspaceStore,
		effectivePath: () => draftSync.draft?.path ?? draftPath,
		// Autosaves landing back on the deployed script become deletes, so
		// reverting edits clears the draft instead of leaving a no-op behind.
		discardIf: (val) => deployedBaseline !== undefined && draftValuesEqual(val, deployedBaseline)
	})

	// Seed from the URL so ScriptBuilder mounts with a populated `initialPath`
	// even when `draftSync.draft` is already defined synchronously from a
	// local autosave. An empty initialPath flips ScriptBuilder's
	// `metadataOpen` heuristic (intended for /scripts/add) into "true" and
	// pops the settings drawer open on /edit.
	let initialPath: string = $state(hash ? '' : (page.params.path ?? ''))

	let scriptBuilder: ScriptBuilder | undefined = $state(undefined)

	let savedScript: Script | NewScript | undefined = $state(undefined)
	let fullyLoaded = $state(false)
	/** Other users (incl. the legacy NULL-email row) with a draft on this path,
	 *  from each `loadScript`. Feeds the AutosaveIndicator's "others" hint. */
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	/** Editor mounted on a per-user draft this load; drives the "Loaded from draft" hint. */
	let loadedFromDraft = $state(false)
	/** "See others' drafts" button state, bound through DraftEditorModals. */
	let othersModalOpen = $state(false)
	/** DraftEditorModals opens the StaleDraftModal when `draftSavedAt < deployedAt`
	 *  (our draft is behind the latest deploy). Cleared between loads to re-fire. */
	let draftSavedAt = $state<string | undefined>(undefined)
	let deployedAt = $state<string | undefined>(undefined)

	// Remounts ScriptBuilder on nav: false while a reload runs, true once data is
	// ready. A synchronous `{#key}` swap instead races Monaco's init against the
	// torn-down container (mirrors how the raw-app editor clears `files`).
	let renderEditor = $state(false)

	let savedPrimarySchedule: ScheduleTrigger | undefined = $state(undefined)

	/** Increments per `loadScript` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadScriptToken = 0
	/** Drops the previous load's `new_draft` strip-on-save listener. */
	let cleanupNewDraftFlag: (() => void) | undefined
	onDestroy(() => cleanupNewDraftFlag?.())
	async function loadScript(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadScriptToken
		fullyLoaded = false
		cleanupNewDraftFlag?.()
		cleanupNewDraftFlag = undefined
		// `?new_draft=true` (from `/scripts/add`'s redirect): a fresh, never-saved
		// `u/{user}/draft_{uuid}` path. Skip the backend fetch (would 404) and seed
		// empty. `path` AND `initialPath` must both be '' so the Path widget's
		// `initPath` calls `reset()`, generating the friendly `<adj>_<kind>` name;
		// any non-empty value is parsed verbatim. Empty `initialPath` also opens the
		// metadata drawer. The flag is stripped only once the first save lands.
		if (page.url.searchParams.get('new_draft') === 'true') {
			// Suspend autosave across the bootstrap: both the seed and
			// ScriptBuilder's `initContent` are programmatic writes that must not
			// post as the user's first edit. ScriptBuilder lifts it in
			// `initContent`'s `.finally` (overlapping the same flag is harmless).
			UserDraft.stopSync('script', draftPath)
			// Page component is reused across same-route nav (e.g. forking from an
			// editor) — clear the previous path's draft-presence state so its hints
			// and stale-draft timestamps don't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// Brand-new script: no deployed baseline, so never discard-on-equal.
			deployedBaseline = undefined
			const templatePath = page.url.searchParams.get('template')
			const hubPath = page.url.searchParams.get('hub')
			const collabLang = page.url.searchParams.get('lang') as ScriptLang | null
			const wacParam = page.url.searchParams.get('wac')
			// Explicit path seed: the fork-a-draft handoff re-homes the source
			// path into the forker's namespace and passes it here.
			const pathParam = page.url.searchParams.get('seed_path')
			const urlScript = decodeUrlScript()
			// Keep `?new_draft=true` until the backend confirms the first autosave,
			// so a refresh before any edit re-seeds here instead of 404-ing on the
			// never-persisted `draft_{uuid}` path.
			cleanupNewDraftFlag = stripNewDraftFlagOnSave({
				workspace: $workspaceStore!,
				itemKind: 'script',
				path: draftPath
			})
			// One-shot YAML/JSON import handoff via $importScriptStore. Consume +
			// clear; imported content is non-empty so ScriptBuilder's template
			// bootstrap (guarded on `content == ''`) leaves it untouched.
			const imported = $importScriptStore
			if (imported) {
				$importScriptStore = undefined
			}
			const empty: EditableScript = {
				path: '',
				summary: '',
				description: '',
				content: '',
				language:
					(wacParam === 'python' ? 'python3' : wacParam === 'typescript' ? 'bun' : null) ??
					collabLang ??
					'bun',
				// MUST be `emptySchema()`, not `{}`: `inferArgs` does
				// `JSON.parse(JSON.stringify(schema.properties))`, and undefined
				// `properties` throws "Could not parse code".
				schema: emptySchema(),
				// Mirrors the backend overlay for deployed=null; ScriptBuilder's Diff
				// button gates on this so draft-only state offers no baseline-less diff.
				no_deployed: true
			} as unknown as EditableScript
			// Seed priority: URL payload > YAML/JSON import > hub > template >
			// wac/lang empty. Empty-`path` seeds get the friendly name; fork seeds
			// carry an explicit `<source>_fork` suggestion.
			let seed: EditableScript = empty
			if (urlScript) {
				seed = {
					...empty,
					...urlScript,
					hash: '',
					extra_perms: {},
					no_deployed: true
				} as unknown as EditableScript
				sendUserToast('Loaded from URL')
			} else if (imported) {
				// Imported fields layer over the empty template; `path` stays '' for
				// the friendly name, editor/deployed keys pinned to new-draft values.
				seed = {
					...empty,
					...imported,
					path: '',
					hash: '',
					extra_perms: {},
					no_deployed: true
				} as unknown as EditableScript
				if (isWorkflowAsCode(imported.content ?? '', imported.language ?? '')) {
					builderTemplate = imported.language === 'python3' ? 'wac_python' : 'wac_typescript'
					sendUserToast('WAC script loaded from YAML/JSON')
				} else {
					sendUserToast('Script loaded from YAML/JSON')
				}
			} else if (hubPath) {
				try {
					const { content, language, summary } = await ScriptService.getHubScriptByPath({
						path: hubPath
					})
					if (tok !== loadScriptToken) return
					seed = {
						...empty,
						description: `Fork of ${hubPath}`,
						content: replaceScriptPlaceholderWithItsValues(hubPath, content),
						summary: summary ?? '',
						language: language as EditableScript['language'],
						path: hubPath + '_fork'
					}
					lockedLanguage = true
				} catch (err: any) {
					if (tok !== loadScriptToken) return
					console.error('Error loading script from hub', err)
					sendUserToast('Error loading script from hub: ' + err.message, true)
				}
			} else if (templatePath) {
				try {
					const template = await ScriptService.getScriptByPath({
						workspace: $workspaceStore!,
						path: templatePath
					})
					if (tok !== loadScriptToken) return
					seed = {
						...empty,
						summary: !emptyString(template.summary) ? `Copy of ${template.summary}` : '',
						description: template.description,
						content: template.content,
						schema: template.schema,
						language: template.language,
						path: template.path + '_fork'
					}
					lockedLanguage = true
				} catch (err: any) {
					if (tok !== loadScriptToken) return
					console.error('Error loading template', err)
					sendUserToast('Error loading template: ' + err.message, true)
				}
			} else if (wacParam === 'python' || wacParam === 'typescript') {
				builderTemplate = wacParam === 'python' ? 'wac_python' : 'wac_typescript'
			}
			if (pathParam) {
				seed = { ...seed, path: pathParam }
			}
			// A seeded path is an explicit choice: parsed verbatim, auto-slug off.
			pathChosen = seed.path !== ''
			initialPath = ''
			savedScript = structuredClone(empty)
			draftSync.draft = seed
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
			// Historical-hash view is read-only relative to drafts (`draftPath` is
			// '' → detached handle), so no baseline is needed.
			deployedBaseline = undefined
			draftSync.draft = { ...scriptByHash, parent_hash: hash, lock: undefined }
		} else {
			const backendScript = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: page.params.path ?? '',
				getDraft
			})
			if (tok !== loadScriptToken) return
			// Backend only computes `other_drafts_users` when `getDraft`. Don't clobber
			// the known list on a `getDraft:false` reload (e.g. reset-to-deployed, which
			// discards only OUR draft and must keep other users' drafts visible).
			if (getDraft) {
				otherDraftsUsers = (backendScript.other_drafts_users ?? []) as OtherDraftUser[]
			}
			// Seed the per-tab `last_sync` so the next autosave attaches a matching
			// timestamp the backend can stale-check. `undefined` (no draft) clears
			// it, making the next save take the "first push" branch.
			draftSync.recordRemoteSync(backendScript.draft_saved_at as string | undefined)
			// Per-response, NOT sticky: navigating to another path in the same editor
			// must reset this, else a later no-own-draft load wrongly enters overlay.
			const hasOwnDraft = !!backendScript.is_draft
			loadedFromDraft = hasOwnDraft
			// Pass both timestamps through for DraftEditorModals' staleness compare:
			// `created_at` is the latest deploy, `draft_saved_at` the draft's save.
			draftSavedAt = backendScript.draft_saved_at as string | undefined
			deployedAt = backendScript.created_at as string | undefined
			// Layer the draft (`.draft`, if any) over the deployed payload at the
			// field level: the draft supplies editor state (content, summary, …),
			// the deployed supplies metadata it lacks (hash, version markers).
			const { draft: draftFromBackend, ...deployedScript } = backendScript as any
			const effectiveScript: EditableScript = draftFromBackend
				? { ...deployedScript, ...draftFromBackend }
				: (deployedScript as EditableScript)
			savedScript = structuredClone($state.snapshot(effectiveScript))
			const parentHash = topHash ?? backendScript.hash
			// Baseline for the autosave `discardIf`: the deployed script with the
			// same `parent_hash` graft the seed below applies, so the unedited draft
			// compares equal. `undefined` when there's no deployed row.
			deployedBaseline = backendScript.no_deployed
				? undefined
				: structuredClone($state.snapshot({ ...deployedScript, parent_hash: parentHash }))
			// "Load another user's draft" handoff: show their value over the
			// deployed metadata. If WE already have a draft → overlay mode (never
			// saved until the user confirms overwriting their own draft).
			const pendingLoad = getDraft
				? OtherUserDraftLoad.takePending($workspaceStore!, 'script', draftPath)
				: undefined
			// Revisiting a path whose overlay was never confirmed/reset (e.g. the user
			// navigated away mid-load): drop the stale lock so editing our own draft
			// works again.
			if (!pendingLoad && OtherUserDraftLoad.isActive($workspaceStore!, 'script', draftPath)) {
				OtherUserDraftLoad.clear($workspaceStore!, 'script', draftPath)
			}
			if (pendingLoad) {
				const loadedValue = {
					...deployedScript,
					...(pendingLoad.value as object),
					parent_hash: parentHash
				} as EditableScript
				if (hasOwnDraft) {
					OtherUserDraftLoad.beginOverlay({
						workspace: $workspaceStore!,
						itemKind: 'script',
						path: draftPath,
						ownerLabel: pendingLoad.ownerLabel,
						loadedValue,
						onResetToOwnDraft: () => loadScript({ getDraft: true })
					})
					// Seed so the bound value updates WITHOUT a POST (the lock would
					// block it anyway, but seeding avoids tripping the edit prompt).
					UserDraft.seed('script', draftPath, loadedValue, { workspace: $workspaceStore! })
				} else {
					// No own draft: adopt their value as ours (autosaves normally).
					draftSync.draft = loadedValue
				}
			} else {
				// `parent_hash` is grafted on so the editor's compile reuses the
				// deployed lock. The first cell write after `acquireEntry` is swallowed
				// by the syncer's seed guard, so this load doesn't POST.
				draftSync.draft = {
					...effectiveScript,
					parent_hash: parentHash
				}
			}
		}

		if (draftSync.draft) {
			initialPath = draftSync.draft.path
			scriptBuilder?.setDraftTriggers(draftSync.draft.draft_triggers)
			scriptBuilder?.setCode(draftSync.draft.content)
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
		if (!savedScript || !$workspaceStore) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		goto(`/scripts/edit/${savedScript.path}`)
		// stopSync-bracketed delete + getDraft:false reload. A bare `remove()` +
		// `loadScript()` loses the race: the reload's draft write re-enters the
		// autosave mirror and displaces the queued `value: null`, so the delete
		// never lands and the draft re-renders.
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'script',
			path: draftPath,
			onResetToDeployed: async () => {
				draftSync.draft = undefined
				await loadScript({ getDraft: false })
			}
		})
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
<!-- Auto-save off: edits aren't persisted on leave, so warn before navigating
	away (and on tab close). When auto-save is on the predicate returns false and
	this modal stays inert — the draft is saved automatically. -->
<UnsavedConfirmationModal
	showAutosaveTips
	hasUnsavedChanges={() =>
		UserDraftDbSyncer.hasUnsavedDisabledChanges({
			workspace: $workspaceStore ?? '',
			itemKind: 'script',
			path: draftPath
		})}
	onDiscardChanges={() =>
		UserDraftDbSyncer.dropPending({
			workspace: $workspaceStore ?? '',
			itemKind: 'script',
			path: draftPath
		})}
/>
<DraftEditorModals
	enabled={!hash}
	workspace={$workspaceStore ?? ''}
	itemKind="script"
	path={page.params.path ?? ''}
	{otherDraftsUsers}
	draftOnly={(savedScript as any)?.no_deployed === true}
	hasOwnDraft={loadedFromDraft}
	onLoadFromServer={() => loadScript()}
	getLocalDraft={() => draftSync.draft}
	bind:othersModalOpen
	{draftSavedAt}
	{deployedAt}
	onLoadLatestDeploy={async () => {
		// stopSync-bracketed; see restoreDeployed for the race.
		if (!$workspaceStore) return
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'script',
			path: draftPath,
			onResetToDeployed: async () => {
				draftSync.draft = undefined
				await loadScript({ getDraft: false })
			}
		})
	}}
/>
{#if draftSync.draft && renderEditor}
	<ScriptBuilder
		bind:this={scriptBuilder}
		{initialPath}
		userDraftPath={draftPath}
		bind:script={draftSync.draft}
		template={builderTemplate}
		{lockedLanguage}
		initialPathChosen={pathChosen}
		{fullyLoaded}
		bind:savedScript
		{initialArgs}
		{diffDrawer}
		{savedPrimarySchedule}
		searchParams={page.url.searchParams}
		{loadedFromDraft}
		othersDraftsCount={otherDraftsUsers.length}
		onOpenOthersDrafts={() => (othersModalOpen = true)}
		onResetToDeployed={async () => {
			draftSync.draft = undefined
			await loadScript({ getDraft: false })
		}}
		onDeploy={(e) => {
			// "Deploy & Stay here" / lib: stay on the editor (just confirm).
			if (e.stay) {
				sendUserToast('Deployed')
				return
			}
			// stopSync-bracketed immediate delete; see restoreDeployed for the race.
			if ($workspaceStore) {
				discardDraftAfterDeploy({
					workspace: $workspaceStore,
					itemKind: 'script',
					path: draftPath
				})
			}
			if ($workspaceStore) invalidate($workspaceStore, 'script')
			goto(`/scripts/get/${e.hash}?workspace=${$workspaceStore}`)
		}}
		onSeeDetails={(e) => {
			goto(`/scripts/get/${e.path}?workspace=${$workspaceStore}`)
		}}
		onNavigate={(item) => goto(editPathFor(item))}
	/>
{/if}
