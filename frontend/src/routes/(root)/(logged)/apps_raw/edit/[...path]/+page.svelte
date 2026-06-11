<script lang="ts">
	import { run } from 'svelte/legacy'

	import { AppService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { readFieldsRecursively } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { HiddenRunnable } from '$lib/components/apps/types'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { page } from '$app/state'
	import { type RawAppData, DEFAULT_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
	import { importStore } from '$lib/components/apps/store'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { usePageDraftSync } from '$lib/components/usePageDraftSync.svelte'
	import { armRestartOnFirstInteraction } from '$lib/userDraftToast'
	import DraftEditorModals from '$lib/components/common/confirmationModal/DraftEditorModals.svelte'
	import { type OtherDraftUser } from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import RawAppTemplatePicker, {
		type RawAppTemplatePickerResult
	} from '$lib/components/raw_apps/RawAppTemplatePicker.svelte'
	import {
		react19Template,
		STARTER_RUNNABLE,
		STARTER_RUNNABLE_KEY
	} from '$lib/components/raw_apps/templates'
	import { aiChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'

	type RawAppDraft = {
		files: Record<string, string>
		runnables: Record<string, any>
		data: RawAppData
		summary: string
		policy?: any
		custom_path?: string
		/** User-typed path the home list renders when set — written by
		 *  the editor when (and only when) the typed path differs from
		 *  the deployed/seeded `savedApp.path`. */
		draft_path?: string
	}

	let files: Record<string, string> | undefined = $state(undefined)
	let runnables = $state({})
	/** Data configuration including tables and creation policy */
	let data: RawAppData = $state({ ...DEFAULT_DATA })
	let newPath = $state('')
	// let lastVersion = 0
	let policy: any = $state({})
	let summary = $state('')
	/** User-typed path surfaced from `RawAppEditorHeader` whenever it
	 *  differs from `savedApp.path`. Mirrored into the autosaved raw-app
	 *  draft below as `draft_path` so the home list can show the friendly
	 *  name instead of the URL's `draft_{uuid}` slot. */
	let pendingDraftPath = $state<string | undefined>(undefined)

	let savedApp:
		| {
				value: {
					files: Record<string, { code: string }>
					runnables: Record<string, HiddenRunnable>
				}
				path: string
				summary: string
				policy: any
				custom_path?: string
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''
	/** Open the framework picker when the user lands on a brand-new draft
	 * (route flag `new_draft=true`). Lets them pick React/Svelte + data
	 * config + optional AI prompt before the editor goes live. */
	let templatePicker = $state(false)

	// Single page-level draft orchestration. `path` is a mount-scoped plain
	// `let` (the editor remounts per path), so this re-keys only on
	// workspace change, matching the prior `UserDraft.use` capture.
	// effectivePath omitted: the live-editor-draft entry is registered by
	// RawAppEditor (`liveEditorDraftStoragePath`).
	const draftSync = usePageDraftSync<RawAppDraft>({
		itemKind: 'raw_app',
		path: () => path,
		workspace: () => $workspaceStore
	})

	// Persist the bundle whenever any of the four pieces of state changes.
	$effect(() => {
		const currentFiles = files
		if (!currentFiles) return
		readFieldsRecursively(currentFiles)
		readFieldsRecursively(runnables)
		readFieldsRecursively(data)
		readFieldsRecursively(policy)
		void summary
		void pendingDraftPath
		draftSync.draft = {
			files: currentFiles,
			runnables,
			data,
			summary,
			policy,
			custom_path: savedApp?.custom_path,
			// Only persist when set — `RawAppEditorHeader` clears it back
			// to undefined once the typed path matches the baseline again,
			// and deploy clears the whole draft, so the field disappears
			// from the saved JSON in both cases.
			...(pendingDraftPath ? { draft_path: pendingDraftPath } : {})
		} as RawAppDraft
	})

	function extractRawApp(app: any) {
		runnables = app.value.runnables
		// Support old formats and new format
		if (app.value.data) {
			const d = app.value.data
			// Handle old nested creation format
			if (d.creation) {
				data = {
					tables: d.tables ?? [],
					datatable: d.creation.datatable,
					schema: d.creation.schema
				}
			} else {
				data = d
			}
		} else if (app.value.datatables) {
			data = { ...DEFAULT_DATA, tables: app.value.datatables }
		}
		files = app.value.files
		summary = app.summary
		// lastVersion = app.version
		policy = app.policy
		// Reload of a draft-only (or rename-in-progress) raw app: prefer
		// the previously-typed `draft_path` so the topbar shows the
		// pending friendly name instead of the autogenerated
		// `u/{user}/draft_{uuid}` URL slot.
		newPath = (app as any).draft_path ?? app.path
	}

	/** Increments per `loadApp` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadAppToken = 0
	/** Mirrors `/apps/edit`'s `isNewApp`: true when no deployed row
	 * exists at the URL path. Flips RawAppEditor's deploy from
	 * `updateApp` to `createApp` so a user-typed path is used. */
	let isNewApp = $state(false)
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	let loadedFromDraft = $state(false)
	let othersModalOpen = $state(false)
	let draftSavedAt = $state<string | undefined>(undefined)
	let deployedAt = $state<string | undefined>(undefined)
	async function loadApp(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadAppToken
		// `?new_draft=true` (set by `/apps_raw/add`'s redirect) means we
		// landed on a fresh `u/{user}/draft_{uuid}` path that's never
		// been saved. Skip the backend fetch (it would 404), seed every
		// piece of state RawAppEditor needs (the template gates rendering
		// on `files`, so an unset files would leave the page blank —
		// which is the bug this branch fixes). Use `path = ''` so the
		// `Path` widget's `initPath` calls `reset()` and generates the
		// friendly `<random_adj>_raw_app` name. Strip the flag last.
		if (page.url.searchParams.get('new_draft') === 'true') {
			isNewApp = true
			// The page component is reused across same-route navigation
			// (e.g. forking from an editor with collaborators) — clear the
			// previous path's draft-presence state so its hints and
			// stale-draft timestamps don't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// Suspend autosave around the new-draft bootstrap: the seed
			// template + the framework picker's `onStart` are both
			// programmatic writes that shouldn't POST as the user's
			// first edit. Resume on the user's first real interaction
			// (which is, in practice, their click on a template card or
			// the picker's X) so the choice is persisted but earlier
			// programmatic mutations aren't.
			if ($workspaceStore) {
				UserDraft.stopSync('raw_app', path, { workspace: $workspaceStore })
				armRestartOnFirstInteraction($workspaceStore, 'raw_app', path)
			}
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			// Backend's `Policy` requires `execution_mode` — an empty
			// object fails to deserialize on deploy.
			const defaultPolicy = {
				on_behalf_of: $userStore?.username.includes('@')
					? $userStore?.username
					: `u/${$userStore?.username}`,
				on_behalf_of_email: $userStore?.email,
				execution_mode: 'publisher'
			} as any
			// One-shot import handoff: "Import from YAML/JSON" for full-code
			// apps (CreateActionsApp) stashes the parsed payload — via
			// $importStore in-memory, or sessionStorage when the /apps_raw
			// route's full page reload (cross-origin isolation) would drop
			// in-memory state — then routes to /apps_raw/add, which
			// redirects here. Wrapped exports carry { summary, value,
			// policy }; bare ones are the value itself.
			let importRaw: any = $importStore
			if ($importStore) {
				$importStore = undefined
			}
			if (!importRaw) {
				const sessionData = sessionStorage.getItem('rawAppImport')
				if (sessionData) {
					sessionStorage.removeItem('rawAppImport')
					try {
						importRaw = JSON.parse(sessionData)
					} catch {
						importRaw = undefined
					}
				}
			}
			const importedValue = importRaw && 'value' in importRaw ? importRaw.value : importRaw
			// Rendering gates on `files` — only honor imports that carry
			// them; otherwise fall through to the template seed.
			if (importedValue?.files) {
				sendUserToast('Loaded from YAML/JSON')
				const importedSummary = ('value' in importRaw ? importRaw.summary : '') ?? ''
				const importedPolicy =
					('value' in importRaw ? importRaw.policy : undefined) ?? defaultPolicy
				savedApp = {
					summary: importedSummary,
					value: importedValue,
					path: '',
					policy: importedPolicy,
					custom_path: undefined
				}
				extractRawApp({ summary: importedSummary, value: importedValue, policy: importedPolicy })
				newPath = ''
				// Imported content IS the starting state — skip the
				// framework picker.
				templatePicker = false
				return
			}
			// Seed the React 19 template up-front so the editor mounts with
			// a usable starting state even if the user dismisses the picker
			// modal without an explicit selection (matches main's /add).
			const seedFiles = { ...react19Template }
			const seedRunnables = { [STARTER_RUNNABLE_KEY]: STARTER_RUNNABLE }
			savedApp = {
				summary: '',
				value: { files: seedFiles as any, runnables: seedRunnables as any },
				path: '',
				policy: defaultPolicy,
				custom_path: undefined
			}
			files = seedFiles
			runnables = seedRunnables
			data = { ...DEFAULT_DATA }
			summary = ''
			policy = defaultPolicy
			newPath = ''
			templatePicker = true
			return
		}
		const backendApp = (await AppService.getAppByPath({
			path: page.params.path ?? '',
			workspace: $workspaceStore!,
			getDraft,
			rawApp: true
		})) as any
		if (tok !== loadAppToken) return
		otherDraftsUsers = (backendApp.other_drafts_users ?? []) as OtherDraftUser[]
		draftSync.recordRemoteSync(backendApp.draft_saved_at as string | undefined)
		isNewApp = !!backendApp.no_deployed
		if (backendApp.is_draft) {
			loadedFromDraft = true
		}
		// Same shape as the regular app route — deploy timestamp lives
		// on `app_version.created_at` (exposed as `backendApp.created_at`).
		// Skip when `no_deployed`; nothing to be older than yet.
		draftSavedAt = backendApp.draft_saved_at as string | undefined
		deployedAt = backendApp.no_deployed ? undefined : (backendApp.created_at as string | undefined)
		// Apply the user's saved draft. The autosave for raw apps writes a
		// flat `RawAppDraft` (`{files, runnables, data, summary, policy,
		// custom_path}`); the backend returns it in `.draft`. The rest of
		// this loader (and `extractRawApp` below) expects the deployed
		// shape where `files`/`runnables`/`data` live under `.value` and
		// the rest are top-level.
		//   - `no_deployed`: no deployed row exists. The response body is
		//     a best-effort stand-in equal to `.draft`. Synthesize the
		//     wrapper so downstream sees the editor's saved state under
		//     `.value` and as top-level metadata.
		//   - deployed + draft: keep the deployed metadata, replace the
		//     editable fields with the saved draft's.
		const savedRawAppDraft = backendApp.draft as
			| {
					files?: any
					runnables?: any
					data?: any
					summary?: string
					policy?: any
					custom_path?: string
					draft_path?: string
			  }
			| undefined
		// Surface the saved `draft_path` on `backendApp` so the
		// extract-from-backend path below seeds `newPath` with the
		// friendly name instead of the URL `draft_{uuid}` slot.
		if (savedRawAppDraft?.draft_path) {
			;(backendApp as any).draft_path = savedRawAppDraft.draft_path
		}
		if (backendApp.no_deployed) {
			backendApp.value = {
				files: savedRawAppDraft?.files ?? {},
				runnables: savedRawAppDraft?.runnables ?? {},
				data: savedRawAppDraft?.data
			}
			backendApp.summary = savedRawAppDraft?.summary ?? ''
			// Backend's `Policy` requires `execution_mode` — fall back
			// to the publisher default when the saved draft (or
			// fetched payload) didn't carry one.
			backendApp.policy = savedRawAppDraft?.policy ?? { execution_mode: 'publisher' }
			backendApp.custom_path = savedRawAppDraft?.custom_path
			backendApp.path = page.params.path ?? ''
		} else if (savedRawAppDraft) {
			backendApp.value = {
				...(backendApp.value ?? {}),
				files: savedRawAppDraft.files ?? backendApp.value?.files ?? {},
				runnables: savedRawAppDraft.runnables ?? backendApp.value?.runnables ?? {},
				data: savedRawAppDraft.data ?? backendApp.value?.data
			}
			if (savedRawAppDraft.summary !== undefined) backendApp.summary = savedRawAppDraft.summary
			if (savedRawAppDraft.policy !== undefined) backendApp.policy = savedRawAppDraft.policy
			if (savedRawAppDraft.custom_path !== undefined)
				backendApp.custom_path = savedRawAppDraft.custom_path
		}
		const backendApp_ = structuredClone(stateSnapshot(backendApp))
		savedApp = {
			summary: backendApp_.summary,
			value: backendApp_.value as any,
			path: backendApp_.path,
			policy: backendApp_.policy,
			custom_path: backendApp_.custom_path
		}
		// Backend canonical: extract the (deployed+draft overlay) raw
		// app into the editor's local pieces. The bundle $effect above
		// re-mirrors them into `draftSync.draft`; the first such
		// write is swallowed by `acquireEntry`'s seed guard so this
		// load doesn't POST.
		extractRawApp(backendApp)
	}

	run(() => {
		// Re-run on workspace OR path change so navigating from one raw app editor
		// to another (e.g. via the workspace picker) reloads the new app.
		const currentPath = page.params.path
		if ($workspaceStore && currentPath !== undefined) {
			// Clear files so RawAppEditor unmounts; it will remount when loadApp
			// completes with fresh data, re-initializing its internal stores.
			files = undefined
			path = currentPath
			loadApp()
		}
	})

	async function restoreDeployed() {
		if (!savedApp) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		draftSync.remove()
		draftSync.draft = undefined
		goto(`/apps/edit/${savedApp.path}`)
		await loadApp()
		redraw++
	}

	let diffDrawer: DiffDrawer | undefined = $state(undefined)

	function onRestore(ev: any) {
		sendUserToast('App restored from previous deployment')
		let prev = ev.detail
		extractRawApp(prev)
		savedApp = {
			summary: prev.summary,
			value: structuredClone(stateSnapshot(prev.value)),
			path: prev.path,
			policy: structuredClone(stateSnapshot(policy)),
			custom_path: prev.custom_path
		}
		redraw++
	}

	function onTemplatePickerStart(result: RawAppTemplatePickerResult, withPrompt: boolean) {
		files = { ...result.files }
		runnables = { ...result.runnables, [STARTER_RUNNABLE_KEY]: STARTER_RUNNABLE }
		data = result.data
		summary = result.summary
		policy = result.policy
		// Remount RawAppEditor so the UI builder iframe picks up the
		// new files instead of leaving the React 19 seed on screen.
		redraw++
		// Sync to aiChatManager so its prompts respect the picked data config.
		aiChatManager.datatableCreationPolicy = {
			enabled: !!result.data.datatable,
			datatable: result.data.datatable,
			schema: result.data.schema
		}
		if (withPrompt && result.prompt) {
			setTimeout(() => {
				aiChatManager.changeMode(AIMode.APP)
				if (!aiChatManager.open) aiChatManager.toggleOpen()
				aiChatManager.instructions = result.prompt!
				aiChatManager.sendRequest()
			}, 500)
		}
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
<DraftEditorModals
	workspace={$workspaceStore ?? ''}
	itemKind="raw_app"
	{path}
	{otherDraftsUsers}
	onLoadFromServer={() => loadApp()}
	getLocalDraft={() => draftSync.draft}
	bind:othersModalOpen
	{draftSavedAt}
	{deployedAt}
	onLoadLatestDeploy={async () => {
		draftSync.draft = undefined
		await loadApp({ getDraft: false })
	}}
/>

<RawAppTemplatePicker bind:open={templatePicker} onStart={onTemplatePickerStart} />

{#if files}
	{#key redraw}
		<div class="h-screen">
			<RawAppEditor
				on:savedNewAppPath={(event) => {
					draftSync.remove()
					goto(`/apps_raw/edit/${event.detail}`)
					newPath = event.detail
				}}
				on:restore={onRestore}
				bind:files
				bind:runnables
				bind:data
				bind:summary
				bind:pendingDraftPath
				{newPath}
				path={page.params.path ?? ''}
				liveEditorDraftStoragePath={path}
				{policy}
				bind:savedApp
				{diffDrawer}
				newApp={isNewApp}
				onResetToDeployed={async () => {
					draftSync.draft = undefined
					await loadApp({ getDraft: false })
				}}
				{loadedFromDraft}
				othersDraftsCount={otherDraftsUsers.length}
				onOpenOthersDrafts={() => (othersModalOpen = true)}
			/>
		</div>
	{/key}
{/if}
