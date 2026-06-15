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
	import { armRestartOnFirstInteraction, runResetToDeployed } from '$lib/userDraftToast'
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
		/** User-typed path the home list renders, set only when it differs
		 *  from the deployed/seeded `savedApp.path`. */
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
	/** User-typed path from `RawAppEditorHeader` when it differs from
	 *  `savedApp.path`; mirrored into the draft below as `draft_path` for the
	 *  home list's friendly name. */
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
				no_deployed?: boolean
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''
	/** Opens the framework picker on a brand-new draft (`new_draft=true`):
	 * React/Svelte + data config + optional AI prompt before the editor goes live. */
	let templatePicker = $state(false)

	// Page-level draft orchestration. `path` is a mount-scoped plain `let` (the
	// editor remounts per path), so this re-keys only on workspace change.
	// effectivePath omitted: the live-editor-draft entry is owned by RawAppEditor.
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
			// Only persist when set, so the field disappears from the saved JSON
			// once the typed path matches the baseline again (or on deploy).
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
		// Prefer the saved `draft_path` so the topbar shows the pending name, not
		// the `draft_{uuid}` URL. See /flows/edit's loader.
		newPath = (app as any).draft_path ?? app.path
	}

	/** Increments per `loadApp` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadAppToken = 0
	/** No deployed row at the URL path; flips RawAppEditor's deploy from
	 * `updateApp` to `createApp`. See /apps/edit's `isNewApp`. */
	let isNewApp = $state(false)
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	let loadedFromDraft = $state(false)
	let othersModalOpen = $state(false)
	let draftSavedAt = $state<string | undefined>(undefined)
	let deployedAt = $state<string | undefined>(undefined)
	async function loadApp(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadAppToken
		// `?new_draft=true` (from `/apps_raw/add`'s redirect): a fresh, never-saved
		// `draft_{uuid}` path. Skip the backend fetch (would 404) and seed every
		// piece of state RawAppEditor needs — rendering gates on `files`, so an
		// unset `files` blanks the page. `path = ''` for the friendly auto-name.
		if (page.url.searchParams.get('new_draft') === 'true') {
			isNewApp = true
			// Page reused across same-route nav: clear the previous path's
			// draft-presence state so it doesn't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// Suspend autosave across the bootstrap: the seed template and the
			// picker's `onStart` are programmatic writes that must not POST as the
			// first edit. Resume on first interaction (the template-card click or
			// picker X) so the choice persists but earlier mutations don't.
			if ($workspaceStore) {
				UserDraft.stopSync('raw_app', path, { workspace: $workspaceStore })
				armRestartOnFirstInteraction($workspaceStore, 'raw_app', path)
			}
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
			// Backend's `Policy` requires `execution_mode` (empty object fails to deserialize).
			const defaultPolicy = {
				on_behalf_of: $userStore?.username.includes('@')
					? $userStore?.username
					: `u/${$userStore?.username}`,
				on_behalf_of_email: $userStore?.email,
				execution_mode: 'publisher'
			} as any
			// Explicit path seed: the fork-a-draft handoff re-homes the source
			// path into the forker's namespace and passes it here.
			const pathParam = page.url.searchParams.get('seed_path')
			// One-shot YAML/JSON import handoff. Carried via $importStore, or
			// sessionStorage when /apps_raw's full page reload would drop in-memory
			// state. Wrapped exports carry { summary, value, policy }; bare ones the value.
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
			// Rendering gates on `files`; only honor imports that carry them.
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
				newPath = pathParam ?? ''
				// Imported content is the starting state — skip the picker.
				templatePicker = false
				return
			}
			// Seed the React 19 template so the editor has a usable state even if the
			// user dismisses the picker without selecting.
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
		// `other_drafts_users` only computed when `getDraft`; don't clobber the
		// known list on a `getDraft:false` reload. See /scripts/edit's loader.
		if (getDraft) {
			otherDraftsUsers = (backendApp.other_drafts_users ?? []) as OtherDraftUser[]
		}
		draftSync.recordRemoteSync(backendApp.draft_saved_at as string | undefined)
		isNewApp = !!backendApp.no_deployed
		if (backendApp.is_draft) {
			loadedFromDraft = true
		}
		// Deploy timestamp is `backendApp.created_at`; skip when `no_deployed`.
		// See /apps/edit's loader.
		draftSavedAt = backendApp.draft_saved_at as string | undefined
		deployedAt = backendApp.no_deployed ? undefined : (backendApp.created_at as string | undefined)
		// The raw-app autosave stores a flat `RawAppDraft`, but this loader (and
		// `extractRawApp`) needs the deployed shape with `files`/`runnables`/`data`
		// under `.value` and the rest top-level. Re-wrap the saved draft (`.draft`):
		//   - `no_deployed`: no deployed row — synthesize the wrapper.
		//   - deployed + draft: keep deployed metadata, swap in the editable fields.
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
		// Surface the saved `draft_path` on `backendApp` so `extractRawApp` seeds
		// `newPath` with the friendly name, not the `draft_{uuid}` URL.
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
			// `execution_mode` required; fall back to publisher when unset.
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
			custom_path: backendApp_.custom_path,
			no_deployed: backendApp_.no_deployed
		}
		// Extract the effective raw app into the editor's local pieces. The bundle
		// $effect re-mirrors them into `draftSync.draft`; the first write is
		// swallowed by `acquireEntry`'s seed guard, so no POST.
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
		if (!savedApp || !$workspaceStore) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		goto(`/apps_raw/edit/${savedApp.path}`)
		// stopSync-bracketed delete + getDraft:false reload; a bare `remove()` +
		// `loadApp()` loses the race. See /scripts/edit's restoreDeployed.
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'raw_app',
			path,
			onResetToDeployed: async () => {
				draftSync.draft = undefined
				await loadApp({ getDraft: false })
				redraw++
			}
		})
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
		// Remount RawAppEditor so the iframe picks up the new files.
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
		// stopSync-bracketed; see /scripts/edit's restoreDeployed for the race.
		if (!$workspaceStore) return
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'raw_app',
			path,
			onResetToDeployed: async () => {
				draftSync.draft = undefined
				await loadApp({ getDraft: false })
			}
		})
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
