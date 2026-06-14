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
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { discardDraftAfterDeploy, runResetToDeployed } from '$lib/userDraftToast'
	import { usePageDraftSync } from '$lib/components/usePageDraftSync.svelte'
	import { importScriptStore } from '$lib/components/scripts/scriptStore.svelte'

	type EditableScript = NewScript & { draft_triggers?: Trigger[] }

	// `initialArgs` is intentionally captured once at mount — it's the
	// session's initial argument set, not per-script. The URL form
	// (`?initial_args=<encoded>`, e.g. the run page's "Fork" action) wins
	// over the store form.
	const urlArgs = page.url.searchParams.get('initial_args')
	let initialArgs = urlArgs ? decodeState(urlArgs) : (get(initialArgsStore) ?? {})
	if (get(initialArgsStore)) $initialArgsStore = undefined

	/** Some pages (run/[...run]'s "Fork" action, workspace_settings'
	 * error/success-handler template buttons) base64-JSON-encode a NewScript
	 * payload into the URL hash of a `/scripts/add` link; the redirect
	 * preserves the hash. That value is an explicit "open this script"
	 * intent and wins over templates, hubs, and YAML imports.
	 *
	 * We can't use `decodeState` from utils.ts directly — it fires its own
	 * "Impossible to parse state" toast on failure, which would noise up the
	 * UI when the hash isn't a script payload at all (e.g. a route anchor).
	 */
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

	// Editor-template seeding state, populated in the `new_draft` branch:
	// `builderTemplate` puts ScriptBuilder in WAC mode for `?wac=` /
	// imported workflows-as-code; `lockedLanguage` pins the language picker
	// for hub/template forks (their content is language-specific);
	// `pathChosen` marks seeds carrying an explicit path so ScriptBuilder's
	// summary→path auto-slug doesn't overwrite it.
	let builderTemplate: 'script' | 'wac_python' | 'wac_typescript' = $state('script')
	let lockedLanguage = $state(false)
	let pathChosen = $state(false)

	// Derived so client-side nav (breadcrumb) re-reads the URL, not mount-time values.
	let topHash = $derived(page.url.searchParams.get('topHash') ?? undefined)

	let hash = $derived(page.url.searchParams.get('hash') ?? undefined)

	// When viewing a specific historical hash we don't want to load or write a
	// local draft — that view is read-only relative to drafts.
	let draftPath = $derived(hash ? '' : (page.params.path ?? ''))

	// Single page-level draft orchestration: the autosave handle (re-keyed
	// on nav via the reactive `draftPath`), the live-editor-draft registry
	// entry, `recordRemoteSync`, and draft removal. `draftSync.draft` stays
	// a stable lvalue for `bind:script`.
	const draftSync = usePageDraftSync<EditableScript>({
		itemKind: 'script',
		path: () => draftPath,
		workspace: () => $workspaceStore,
		effectivePath: () => draftSync.draft?.path ?? draftPath
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
	/** Other workspace users (and the legacy NULL-email row, if any) with
	 *  a draft on this path. Populated from the deployed-overlay response
	 *  on each `loadScript`; the AutosaveIndicator picks up the count for
	 *  its on-mount "Others are working on this script" hint. */
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	/** Whether the editor mounted on a per-user draft this load — flipped
	 *  true once when the overlay response says so, drives the
	 *  AutosaveIndicator's on-mount "Loaded from draft" hint. */
	let loadedFromDraft = $state(false)
	/** Bound through DraftEditorModals; flipped on from the
	 *  AutosaveIndicator popover's "See others' drafts" button. */
	let othersModalOpen = $state(false)
	/** Timestamps DraftEditorModals compares to decide whether to open
	 *  the StaleDraftModal: when `draftSavedAt < deployedAt` the
	 *  authed user has been editing a draft that's now behind the
	 *  latest deployed version. Cleared between loads so the modal
	 *  re-fires when a fresh load surfaces fresh staleness. */
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
			// Suspend autosave for the whole new-draft bootstrap: the seed
			// AND ScriptBuilder's `initContent` (which fills
			// `script.content` from a template) are both programmatic
			// writes that shouldn't appear on the server as the user's
			// first edit. ScriptBuilder lifts the suspension in
			// `initContent`'s `.finally`; this overlap is harmless (both
			// calls set the same flag).
			UserDraft.stopSync('script', draftPath)
			// The page component is reused across same-route navigation
			// (e.g. forking from an editor with collaborators) — clear the
			// previous path's draft-presence state so its hints and
			// stale-draft timestamps don't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// Capture every seeding intent BEFORE touching the URL — all of
			// these used to be consumed by /scripts/add and are preserved
			// verbatim by its redirect.
			const templatePath = page.url.searchParams.get('template')
			const hubPath = page.url.searchParams.get('hub')
			const collabLang = page.url.searchParams.get('lang') as ScriptLang | null
			const wacParam = page.url.searchParams.get('wac')
			// Explicit path seed — the fork-a-draft handoff re-homes the
			// source path into the forker's namespace and passes it here.
			const pathParam = page.url.searchParams.get('seed_path')
			const urlScript = decodeUrlScript()
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			// One-shot import handoff: "Import workflows-as-code from
			// YAML/JSON" (CreateActionsFlow) writes $importScriptStore and
			// routes to /scripts/add, which redirects here. Consume + clear;
			// imported content is non-empty, so ScriptBuilder's
			// template-seeding bootstrap (guarded on `content == ''`)
			// leaves it untouched.
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
				// MUST be `emptySchema()` (`{properties: {}, required: [],
				// type: 'object'}`), NOT `{}`. `inferArgs` does
				// `JSON.parse(JSON.stringify(schema.properties))` —
				// `JSON.stringify(undefined)` returns the value `undefined`,
				// and `JSON.parse(undefined)` coerces to the literal string
				// "undefined", throwing the toast "Could not parse code".
				schema: emptySchema(),
				// Mirrors the backend's overlay shape for deployed=null
				// paths — ScriptBuilder's Diff button gates on this so
				// /add (and any other draft-only state) doesn't offer a
				// diff that has no baseline to compare against.
				no_deployed: true
			} as unknown as EditableScript
			// Seed selection, in main's /scripts/add priority order:
			// explicit URL-hash payload > YAML/JSON import > hub fork >
			// template fork > wac/lang-flavored empty. Seeds with an empty
			// `path` let the Path widget generate the friendly name; fork
			// seeds carry an explicit `<source>_fork` suggestion instead.
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
				// Imported fields layer over the empty template; `path` stays
				// '' so the Path widget still generates the friendly name, and
				// the editor-only/deployed-only keys are pinned to new-draft
				// values.
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
			// A seeded path (?path=, hub/template forks, URL payloads) is an
			// explicit choice — the Path widget parses it verbatim and the
			// summary auto-slug must leave it alone.
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
			draftSync.draft = { ...scriptByHash, parent_hash: hash, lock: undefined }
		} else {
			const backendScript = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: page.params.path ?? '',
				getDraft
			})
			if (tok !== loadScriptToken) return
			// The backend only computes `other_drafts_users` when `getDraft`
			// is true (it skips the cross-user lookup otherwise). Don't clobber
			// the known list to empty on a `getDraft:false` reload — e.g.
			// reset-to-deployed, which discards only OUR draft and leaves other
			// users' drafts (and the "See others' drafts" button) intact.
			if (getDraft) {
				otherDraftsUsers = (backendScript.other_drafts_users ?? []) as OtherDraftUser[]
			}
			// Seed the per-tab `last_sync` map with the server's draft
			// timestamp so the next autosave attaches a matching
			// `last_sync` and the backend can reject stale writes.
			// `undefined` (no draft existed) clears the entry — the
			// next save then takes the "first push" branch on the server.
			draftSync.recordRemoteSync(backendScript.draft_saved_at as string | undefined)
			if (backendScript.is_draft) {
				loadedFromDraft = true
			}
			// Latest deploy's created_at on `script` is the row's
			// `created_at`; the per-user draft's save time is
			// `draft_saved_at`. DraftEditorModals takes the comparison
			// from here, so we just pass both through.
			draftSavedAt = backendScript.draft_saved_at as string | undefined
			deployedAt = backendScript.created_at as string | undefined
			// `backendScript` is the deployed payload; the user's saved
			// draft (if any) sits in `.draft`. Layer the draft over the
			// deployed at the field level — the draft contributes editor
			// state (content, summary, …) and the deployed contributes
			// metadata the draft never carries (hash, version markers).
			const { draft: draftFromBackend, ...deployedScript } = backendScript as any
			const effectiveScript: EditableScript = draftFromBackend
				? { ...deployedScript, ...draftFromBackend }
				: (deployedScript as EditableScript)
			savedScript = structuredClone($state.snapshot(effectiveScript))
			// Backend is canonical: write the baked baseline into the
			// cell. `parent_hash` is grafted on so the editor's compile
			// reuses the deployed lock. The first cell write after
			// `acquireEntry` is swallowed by the syncer's seed guard, so
			// this load doesn't POST.
			draftSync.draft = {
				...effectiveScript,
				parent_hash: topHash ?? backendScript.hash
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
		// stopSync-bracketed delete + getDraft:false reload — same dance as
		// the AutosaveIndicator's reset. A bare `remove()` + `loadScript()`
		// would lose the race: the reload's draft write re-enters the
		// autosave mirror and displaces the queued `value: null`, so the
		// delete never lands and the editor re-renders the draft.
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
<DraftEditorModals
	enabled={!hash}
	workspace={$workspaceStore ?? ''}
	itemKind="script"
	path={page.params.path ?? ''}
	{otherDraftsUsers}
	onLoadFromServer={() => loadScript()}
	getLocalDraft={() => draftSync.draft}
	bind:othersModalOpen
	{draftSavedAt}
	{deployedAt}
	onLoadLatestDeploy={async () => {
		// Bracketed like the AutosaveIndicator reset — an unbracketed
		// delete gets displaced by the reload's deployed-payload write,
		// leaving a deployed-identical draft behind.
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
			// stopSync-bracketed immediate delete — a bare remove() only
			// queues the null and post-deploy mirror writes can displace it.
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
