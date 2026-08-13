<script lang="ts">
	import { onDestroy, untrack } from 'svelte'
	import { stripNewDraftFlag, stripNewDraftFlagOnSave, shouldSeedNewDraft } from '$lib/newDraftFlag'

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
	import {
		type RawAppData,
		DEFAULT_DATA,
		extractDataConfig
	} from '$lib/components/raw_apps/dataTableRefUtils'
	import { importStore } from '$lib/components/apps/store'
	import { UserDraft, draftValuesEqual } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { usePageDraftSync } from '$lib/components/usePageDraftSync.svelte'
	import { OtherUserDraftLoad } from '$lib/components/otherUserDraftLoad.svelte'
	import { armRestartOnFirstInteraction, runResetToDeployed } from '$lib/userDraftToast'
	import DraftEditorModals from '$lib/components/common/confirmationModal/DraftEditorModals.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
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
	let labels = $state<string[] | undefined>(undefined)
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
				labels?: string[]
				no_deployed?: boolean
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''
	/** Opens the framework picker on a brand-new draft (`new_draft=true`):
	 * React/Svelte + data config + optional AI prompt before the editor goes live. */
	let templatePicker = $state(false)

	/** Deployed raw-app bundle this load, the baseline the autosave `discardIf`
	 * compares against. `undefined` for draft-only paths so they never
	 * self-destruct by matching a non-existent baseline. */
	let deployedBaseline = $state<RawAppDraft | undefined>(undefined)

	// Page-level draft orchestration. Keyed on the REACTIVE `page.params.path`,
	// not the mount-scoped `path` `let`: this page is not remounted on a
	// same-route navigation, so the post-deploy `goto` (draft_{uuid} → the
	// chosen path) must re-key the autosave handle onto the new path. Keying on
	// the non-reactive `path` left the handle stuck on the old draft path, so
	// edits to the just-deployed app autosaved to a dead key (autosave appeared
	// broken). See /scripts/edit, which derives its draft path the same way.
	// effectivePath omitted: the live-editor-draft entry is owned by RawAppEditor.
	const draftSync = usePageDraftSync<RawAppDraft>({
		itemKind: 'raw_app',
		path: () => page.params.path ?? '',
		workspace: () => $workspaceStore,
		// Autosaves landing back on the deployed raw app become deletes, so
		// reverting edits clears the draft instead of leaving a no-op behind.
		discardIf: (val) => deployedBaseline !== undefined && draftValuesEqual(val, deployedBaseline)
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
			// Persist the typed path as `draft_path` only when it actually differs
			// from the current path — a `draft_path` equal to the baseline is a
			// no-op that would block the draft from deduping against the deployed
			// app (which carries none). Drops back out on a revert or deploy.
			...(pendingDraftPath && pendingDraftPath !== (savedApp?.path ?? '')
				? { draft_path: pendingDraftPath }
				: {})
		} as RawAppDraft
	})

	/** Normalize a raw-app `value` into the editor's `data` config, supporting
	 * the old nested `creation` / `datatables` shapes. `undefined` when the
	 * value carries no data config (caller keeps the current/default `data`). */
	function extractRawApp(app: any) {
		runnables = app.value.runnables
		// Support old formats and new format
		const extractedData = extractDataConfig(app.value)
		if (extractedData) data = extractedData
		files = app.value.files
		summary = app.summary
		labels = app.labels
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
	/** Drops the previous load's `new_draft` strip-on-save listener. */
	let cleanupNewDraftFlag: (() => void) | undefined
	onDestroy(() => cleanupNewDraftFlag?.())
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
		cleanupNewDraftFlag?.()
		cleanupNewDraftFlag = undefined
		// `?new_draft=true` (from `/apps_raw/add`'s redirect): a fresh, never-saved
		// `draft_{uuid}` path. Skip the backend fetch (would 404) and seed every
		// piece of state RawAppEditor needs — rendering gates on `files`, so an
		// unset `files` blanks the page. `path = ''` for the friendly auto-name.
		// Skip the seed branch once the draft is persisted this session so a stale
		// `?new_draft` loads the saved draft instead of blanking it — see
		// shouldSeedNewDraft.
		if (shouldSeedNewDraft(page.url.searchParams, $workspaceStore, 'raw_app', path)) {
			isNewApp = true
			// Page reused across same-route nav: clear the previous path's
			// draft-presence state so it doesn't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// `labels` is route-level state; reset it too so a fresh draft doesn't
			// inherit (and then deploy) the previously-opened app's labels. The
			// import branch re-seeds it via extractRawApp below.
			labels = undefined
			// Brand-new raw app: no deployed baseline, so never discard-on-equal.
			deployedBaseline = undefined
			// Suspend autosave across the bootstrap: the seed template and the
			// picker's `onStart` are programmatic writes that must not POST as the
			// first edit. Resume on first interaction (the template-card click or
			// picker X) so the choice persists but earlier mutations don't.
			if ($workspaceStore) {
				UserDraft.stopSync('raw_app', path, { workspace: $workspaceStore })
				armRestartOnFirstInteraction($workspaceStore, 'raw_app', path)
				// Keep `?new_draft=true` until the backend confirms the first autosave,
				// so a refresh before any edit re-seeds here instead of 404-ing on the
				// never-persisted `draft_{uuid}` path.
				cleanupNewDraftFlag = stripNewDraftFlagOnSave({
					workspace: $workspaceStore,
					itemKind: 'raw_app',
					path
				})
			}
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
		// Falling through with `?new_draft=true` still set means the draft is
		// already persisted (see shouldSeedNewDraft) — drop the now-meaningless
		// flag so it doesn't linger in the URL / remembered nav route.
		stripNewDraftFlag()
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
		// Per-response, NOT sticky: a later no-own-draft load in the same editor
		// must reset this so it can't wrongly force overlay mode.
		const hasOwnDraft = !!backendApp.is_draft
		loadedFromDraft = hasOwnDraft
		// Deploy timestamp is `backendApp.created_at`; skip when `no_deployed`.
		// See /apps/edit's loader.
		draftSavedAt = backendApp.draft_saved_at as string | undefined
		deployedAt = backendApp.no_deployed ? undefined : (backendApp.created_at as string | undefined)
		// Deployed baseline for the autosave `discardIf`, captured BEFORE the swap
		// below mutates `backendApp`. Mirrors the bundle `$effect`'s shape (minus
		// the edit-only `draft_path`) so an unedited draft compares equal.
		// `undefined` when there's no deployed row.
		deployedBaseline = backendApp.no_deployed
			? undefined
			: (structuredClone(
					stateSnapshot({
						files: backendApp.value?.files ?? {},
						runnables: backendApp.value?.runnables ?? {},
						data: extractDataConfig(backendApp.value) ?? { ...DEFAULT_DATA },
						summary: backendApp.summary ?? '',
						policy: backendApp.policy,
						custom_path: backendApp.custom_path
					})
				) as RawAppDraft)
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
			labels: backendApp_.labels,
			no_deployed: backendApp_.no_deployed
		}
		// Extract the effective raw app into the editor's local pieces. The bundle
		// $effect re-mirrors them into `draftSync.draft`; the first write is
		// swallowed by `acquireEntry`'s seed guard, so no POST.
		extractRawApp(backendApp)
		// "Load another user's draft" handoff: their value is a flat RawAppDraft
		// bundle. Override the local pieces with it; overlay mode (we have our own
		// draft) hard-locks saves until the user confirms overwriting. The bundle
		// $effect's write to `draftSync.draft` is blocked by the lock. See /scripts.
		const pendingLoad = getDraft
			? OtherUserDraftLoad.takePending($workspaceStore!, 'raw_app', path)
			: undefined
		// Revisiting a path whose overlay was never confirmed/reset: drop the stale
		// lock so editing our own draft works again. See /scripts/edit's loader.
		if (!pendingLoad && OtherUserDraftLoad.isActive($workspaceStore!, 'raw_app', path)) {
			OtherUserDraftLoad.clear($workspaceStore!, 'raw_app', path)
		}
		if (pendingLoad) {
			const v = pendingLoad.value as RawAppDraft
			files = v.files ?? {}
			runnables = v.runnables ?? {}
			data = v.data ?? { ...DEFAULT_DATA }
			summary = v.summary ?? ''
			policy = v.policy ?? {}
			newPath = v.draft_path ?? savedApp?.path ?? path
			if (hasOwnDraft) {
				OtherUserDraftLoad.beginOverlay({
					workspace: $workspaceStore!,
					itemKind: 'raw_app',
					path,
					ownerLabel: pendingLoad.ownerLabel,
					// Mirror the bundle the persist-$effect produces from these pieces,
					// so the cascade write that re-mirrors them isn't seen as an edit.
					loadedValue: {
						files,
						runnables,
						data,
						summary,
						policy,
						custom_path: savedApp?.custom_path,
						...(pendingDraftPath ? { draft_path: pendingDraftPath } : {})
					} as RawAppDraft,
					onResetToOwnDraft: () => loadApp({ getDraft: true })
				})
			}
		}
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one raw app editor
		// to another (e.g. via the workspace picker) reloads the new app.
		const currentPath = page.params.path
		if ($workspaceStore && currentPath !== undefined) {
			// untrack so loadApp's reactive reads (the draft-hint SvelteMap via
			// shouldSeedNewDraft) don't subscribe this effect — else the first
			// autosave's optimistic hint flip re-fires loadApp mid-bootstrap and
			// 404s on the not-yet-POSTed draft. Depend only on path/workspace above.
			untrack(() => {
				// Clear files so RawAppEditor unmounts; it remounts when loadApp
				// completes with fresh data, re-initializing its internal stores.
				files = undefined
				path = currentPath
				loadApp()
			})
		}
	})

	// Single source for "reset the editor to the deployed app": drop the live
	// draft handle and reload without the draft overlay. Shared by the diff
	// drawer's restore, the conflict modal's "load latest deploy", and the
	// AutosaveIndicator's "Reset to deployed" so the three can't drift. A stray
	// `redraw++` here previously remounted RawAppEditor mid-reset (inside the
	// runResetToDeployed stopSync bracket), and the fresh mount's draft write
	// resurrected the draft — so only the diff-drawer restore appeared to do
	// nothing while the AutosaveIndicator path (no remount) worked.
	async function reloadDeployed() {
		draftSync.draft = undefined
		await loadApp({ getDraft: false })
	}

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
			onResetToDeployed: reloadDeployed
		})
	}

	let diffDrawer: DiffDrawer | undefined = $state(undefined)

	function onRestore(restoredApp: any) {
		sendUserToast('App restored from previous deployment')
		let prev = restoredApp
		extractRawApp(prev)
		savedApp = {
			summary: prev.summary,
			value: structuredClone(stateSnapshot(prev.value)),
			path: prev.path,
			policy: structuredClone(stateSnapshot(policy)),
			custom_path: prev.custom_path,
			labels: prev.labels
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
<!-- Auto-save off: edits aren't persisted on leave, so warn before navigating
	away (and on tab close). Inert while auto-save is on. -->
<UnsavedConfirmationModal
	showAutosaveTips
	hasUnsavedChanges={() =>
		UserDraftDbSyncer.hasUnsavedDisabledChanges({
			workspace: $workspaceStore ?? '',
			itemKind: 'raw_app',
			path
		})}
	onDiscardChanges={() =>
		UserDraftDbSyncer.dropPending({
			workspace: $workspaceStore ?? '',
			itemKind: 'raw_app',
			path
		})}
/>
<DraftEditorModals
	workspace={$workspaceStore ?? ''}
	itemKind="raw_app"
	{path}
	{otherDraftsUsers}
	draftOnly={isNewApp}
	hasOwnDraft={loadedFromDraft}
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
			onResetToDeployed: reloadDeployed
		})
	}}
/>

<RawAppTemplatePicker bind:open={templatePicker} onStart={onTemplatePickerStart} />

{#if files}
	{#key redraw}
		<div class="h-screen">
			<RawAppEditor
				onSavedNewAppPath={(savedPath) => {
					draftSync.remove()
					goto(`/apps_raw/edit/${savedPath}`)
					newPath = savedPath
				}}
				{onRestore}
				bind:files
				bind:runnables
				bind:data
				bind:summary
				bind:pendingDraftPath
				{newPath}
				{labels}
				path={page.params.path ?? ''}
				liveEditorDraftStoragePath={path}
				{policy}
				bind:savedApp
				{diffDrawer}
				newApp={isNewApp}
				onResetToDeployed={reloadDeployed}
				{loadedFromDraft}
				othersDraftsCount={otherDraftsUsers.length}
				onOpenOthersDrafts={() => (othersModalOpen = true)}
			/>
		</div>
	{/key}
{/if}
