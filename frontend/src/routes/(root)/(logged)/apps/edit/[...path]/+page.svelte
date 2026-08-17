<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, type AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { App } from '$lib/components/apps/types'
	import { migrateApp } from '$lib/components/apps/migrateApp'
	import DraftEditorModals from '$lib/components/common/confirmationModal/DraftEditorModals.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { type OtherDraftUser } from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { emptyApp } from '$lib/components/apps/editor/appUtils'
	import { importStore } from '$lib/components/apps/store'
	import { onDestroy, tick, untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { stripNewDraftFlag, stripNewDraftFlagOnSave, shouldSeedNewDraft } from '$lib/newDraftFlag'
	import { OtherUserDraftLoad } from '$lib/components/otherUserDraftLoad.svelte'
	import { runResetToDeployed } from '$lib/userDraftToast'

	let app = $state(undefined as (AppWithLastVersion & { value: any }) | undefined)
	let appEditor: AppEditor | undefined = $state(undefined)
	/** Seeded from a hub app this load; AppEditor relaxes a few authoring affordances. */
	let fromHub = $state(false)
	let savedApp:
		| {
				value: App
				path: string
				summary: string
				policy: any
				custom_path?: string
				labels?: string[]
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''
	/** No deployed app at the URL path. Drives the editor's deploy:
	 * `createApp` vs `updateApp`. Flips false once a deploy lands here. */
	let isNewApp = $state(false)
	/** Deployed app value this load, the baseline AppEditor's autosave
	 * `discardIf` compares against. `undefined` for draft-only paths so they
	 * never self-destruct by matching a non-existent baseline. */
	let deployedBaseline = $state<App | undefined>(undefined)
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	let loadedFromDraft = $state(false)
	let othersModalOpen = $state(false)
	let draftSavedAt = $state<string | undefined>(undefined)
	let deployedAt = $state<string | undefined>(undefined)
	// The app_version the draft was forked from (pinned), + the deployed head, for
	// the precise staleness check in DraftEditorModals (vs the drifting timestamp).
	let draftBaseVersion = $state<number | undefined>(undefined)
	let deployedHeadVersion = $state<number | undefined>(undefined)

	/** Increments per `loadApp` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadAppToken = 0
	/** Drops the previous load's `new_draft` strip-on-save listener. */
	let cleanupNewDraftFlag: (() => void) | undefined
	onDestroy(() => cleanupNewDraftFlag?.())
	async function loadApp(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadAppToken
		cleanupNewDraftFlag?.()
		cleanupNewDraftFlag = undefined
		// `?new_draft=true` (from `/apps/add`'s redirect): a fresh, never-saved
		// `draft_{uuid}` path. Skip the backend fetch (would 404) and seed empty
		// with `path = ''` for the friendly auto-name. See /scripts/edit's loader.
		// Skip the seed branch once the draft is persisted this session so a stale
		// `?new_draft` loads the saved draft instead of blanking it — see
		// shouldSeedNewDraft.
		if (shouldSeedNewDraft(page.url.searchParams, $workspaceStore, 'app', path)) {
			// Suspend autosave across the bootstrap: the seed assignment and
			// AppEditor's `firstMirror` are programmatic writes that must not POST
			// as the first edit. AppEditor lifts it in `onMount`.
			UserDraft.stopSync('app', page.params.path ?? '')
			// Deploy must `createApp` at the user-typed path, not `updateApp` at the URL.
			isNewApp = true
			// Page reused across same-route nav: clear the previous path's
			// draft-presence state so it doesn't bleed onto the fresh draft.
			otherDraftsUsers = []
			loadedFromDraft = false
			draftSavedAt = undefined
			deployedAt = undefined
			// New-draft returns before the version assignment below, so clear the
			// previous app's version-staleness inputs, else they bleed across the
			// reused route and falsely trip the stale-draft modal.
			draftBaseVersion = undefined
			deployedHeadVersion = undefined
			// Brand-new app: no deployed baseline, so never discard-on-equal.
			deployedBaseline = undefined
			const templatePath = page.url.searchParams.get('template')
			const templateId = page.url.searchParams.get('template_id')
			const hubId = page.url.searchParams.get('hub')
			// Explicit path seed: the fork-a-draft handoff re-homes the source
			// path into the forker's namespace and passes it here.
			const pathParam = page.url.searchParams.get('seed_path')
			// Keep `?new_draft=true` until the backend confirms the first autosave,
			// so a refresh before any edit re-seeds here instead of 404-ing on the
			// never-persisted `draft_{uuid}` path.
			cleanupNewDraftFlag = stripNewDraftFlagOnSave({
				workspace: $workspaceStore!,
				itemKind: 'app',
				path
			})
			// Backend's `Policy` requires `execution_mode` (empty object fails to
			// deserialize on deploy); `publisher` is the default authoring mode.
			const emptyPolicy = { execution_mode: 'publisher' } as any
			// One-shot import handoff via $importStore (YAML/JSON or "Build app").
			// Wrapped exports carry { summary, value, policy }; bare ones are the App value.
			const importRaw = $importStore
			if (importRaw) {
				$importStore = undefined
				sendUserToast('Loaded from YAML/JSON')
			}
			// Seed priority: YAML/JSON import > template > template-version > hub > empty.
			let seedSummary = ''
			let seedValue = emptyApp() as any
			let seedPolicy = emptyPolicy
			if (importRaw) {
				if ('value' in importRaw) {
					seedSummary = importRaw.summary ?? ''
					seedValue = importRaw.value
					seedPolicy = importRaw.policy ?? emptyPolicy
				} else {
					seedValue = importRaw
				}
			} else if (templatePath) {
				try {
					const template = await AppService.getAppByPath({
						workspace: $workspaceStore!,
						path: templatePath
					})
					if (tok !== loadAppToken) return
					seedValue = template.value
					sendUserToast('App loaded from template')
				} catch (err: any) {
					if (tok !== loadAppToken) return
					console.error('Error loading template', err)
					sendUserToast('Error loading template: ' + (err.body ?? err.message), true)
				}
			} else if (templateId) {
				try {
					const template = await AppService.getAppByVersion({
						workspace: $workspaceStore!,
						id: parseInt(templateId)
					})
					if (tok !== loadAppToken) return
					seedValue = template.value
					sendUserToast('App loaded from template')
				} catch (err: any) {
					if (tok !== loadAppToken) return
					console.error('Error loading template', err)
					sendUserToast('Error loading template: ' + (err.body ?? err.message), true)
				}
			} else if (hubId) {
				try {
					const hub = await AppService.getHubAppById({ id: Number(hubId) })
					if (tok !== loadAppToken) return
					seedValue = {
						hiddenInlineScripts: [],
						unusedInlineScripts: [],
						fullscreen: false,
						...((hub.app.value ?? {}) as any)
					}
					seedSummary = hub.app.summary
					fromHub = true
					sendUserToast('App loaded from Hub')
				} catch (err: any) {
					if (tok !== loadAppToken) return
					console.error('Error loading hub app', err)
					sendUserToast('Error loading hub app: ' + (err.body ?? err.message), true)
				}
			}
			app = {
				summary: seedSummary,
				value: seedValue,
				path: pathParam ?? '',
				policy: seedPolicy,
				custom_path: undefined,
				versions: [] as any,
				id: 0 as any,
				extra_perms: {},
				created_at: new Date().toISOString(),
				created_by: '',
				raw_app: false
			} as unknown as AppWithLastVersion & { value: any }
			savedApp = {
				summary: seedSummary,
				value: seedValue,
				path: pathParam ?? '',
				policy: seedPolicy
			}
			// Tutorial links ("/apps/add?tutorial=...") land here via the
			// redirect; fire once AppEditor has mounted and the runnable
			// panel the tour points at exists.
			const tutorialParam = page.url.searchParams.get('tutorial')
			if (tutorialParam) {
				await tick()
				let attempts = 0
				while (attempts < 20 && !document.querySelector('#app-editor-runnable-panel')) {
					await new Promise((resolve) => setTimeout(resolve, 100))
					attempts++
				}
				if (tok !== loadAppToken) return
				appEditor?.triggerTutorial()
			}
			return
		}
		// Falling through with `?new_draft=true` still set means the draft is
		// already persisted (see shouldSeedNewDraft) — drop the now-meaningless
		// flag so it doesn't linger in the URL / remembered nav route.
		stripNewDraftFlag()
		let backendApp = await AppService.getAppByPath({
			path: page.params.path ?? '',
			workspace: $workspaceStore!,
			getDraft
		})
		if (tok !== loadAppToken) return
		// Deployed App value for AppEditor's autosave `discardIf`, captured BEFORE
		// the draft swap below replaces `backendApp.value`. `undefined` when
		// there's no deployed row (draft-only path).
		deployedBaseline = backendApp.no_deployed
			? undefined
			: // Carry the deployed summary onto the baseline: the autosave mirrors the
				// summary onto the live App value, so the `discardIf` no-op comparison must
				// see the deployed summary here too — otherwise a reverted-to-deployed draft
				// never compares equal (and a summary-only edit still counts as a change).
				({
					...(structuredClone(stateSnapshot(backendApp.value)) as App),
					summary: backendApp.summary
				} as App)
		// `other_drafts_users` only computed when `getDraft`; don't clobber the
		// known list on a `getDraft:false` reload. See /scripts/edit's loader.
		if (getDraft) {
			otherDraftsUsers = (backendApp.other_drafts_users ?? []) as OtherDraftUser[]
		}
		if ($workspaceStore && path) {
			UserDraftDbSyncer.recordRemoteSync(
				{ workspace: $workspaceStore, itemKind: 'app', path },
				backendApp.draft_saved_at
			)
		}
		// The app autosave stores a raw `App`, but this loader (and AppEditor's
		// `app={app.value}` prop) needs the `AppWithLastVersion` wrapper. Wrap the
		// saved draft (`.draft`) back into that shape:
		//   - `no_deployed`: no deployed row — synthesize a wrapper with empty
		//     deployed-metadata defaults and the saved App as `.value`.
		//   - deployed + draft: keep deployed metadata, swap in the saved App.
		const savedDraftApp = (backendApp as any).draft as App | undefined
		// Re-evaluate per load: true for draft-only paths, false once deployed.
		isNewApp = !!backendApp.no_deployed
		if (backendApp.no_deployed) {
			backendApp = {
				// Draft-only app: the summary rides on the autosaved App value
				// (no deployed column to read it from).
				summary: savedDraftApp?.summary ?? '',
				value: (savedDraftApp ?? {}) as App,
				path: page.params.path ?? '',
				// `execution_mode` required; matches the new-app seed above.
				policy: { execution_mode: 'publisher' } as any,
				custom_path: undefined,
				versions: undefined as any,
				id: 0 as any,
				extra_perms: {},
				created_at: new Date().toISOString(),
				created_by: '',
				raw_app: false,
				is_draft: true,
				draft_saved_at: backendApp.draft_saved_at,
				no_deployed: true
			} as unknown as typeof backendApp
		} else if (savedDraftApp) {
			// Deployed app with a draft: swap in the draft value and honor a draft
			// summary edit (falls back to the deployed summary when the draft has none).
			backendApp = {
				...backendApp,
				value: savedDraftApp,
				summary: savedDraftApp.summary ?? backendApp.summary
			} as typeof backendApp
		}
		// Per-response, NOT sticky: a later no-own-draft load in the same editor
		// must reset this so it can't wrongly force overlay mode.
		const hasOwnDraft = !!backendApp.is_draft
		loadedFromDraft = hasOwnDraft
		// Pass both timestamps for DraftEditorModals' staleness compare: `created_at`
		// is the deploy time, `draft_saved_at` the draft's. Skip `deployedAt` when
		// `no_deployed` — no baseline to be older than.
		draftSavedAt = backendApp.draft_saved_at as string | undefined
		deployedAt = backendApp.no_deployed ? undefined : (backendApp.created_at as string | undefined)
		// `parent_version` rides on the persisted draft (pinned at fork); undefined
		// for a pre-feature draft. Head = the last entry of the deployed `versions`.
		draftBaseVersion = savedDraftApp?.parent_version
		deployedHeadVersion =
			backendApp.no_deployed || !backendApp.versions
				? undefined
				: backendApp.versions[backendApp.versions.length - 1]
		const backendApp_ = structuredClone(stateSnapshot(backendApp))
		savedApp = {
			summary: backendApp_.summary,
			value: backendApp_.value as App,
			path: backendApp_.path,
			policy: backendApp_.policy,
			custom_path: backendApp_.custom_path,
			labels: backendApp_.labels
		}
		// "Load another user's draft" handoff: render their value. Overlay mode (we
		// have our own draft) hard-locks saves until the user confirms overwriting
		// it (AppEditor's own autosave is blocked by the lock). See /scripts/edit.
		const pendingLoad = getDraft
			? OtherUserDraftLoad.takePending($workspaceStore!, 'app', path)
			: undefined
		// Revisiting a path whose overlay was never confirmed/reset: drop the stale
		// lock so editing our own draft works again. See /scripts/edit's loader.
		if (!pendingLoad && OtherUserDraftLoad.isActive($workspaceStore!, 'app', path)) {
			OtherUserDraftLoad.clear($workspaceStore!, 'app', path)
		}
		if (pendingLoad) {
			backendApp = { ...backendApp, value: pendingLoad.value as App } as typeof backendApp
			if (hasOwnDraft) {
				// AppEditor `migrateApp`s the value in place on mount (see its
				// `migratedDeployedBaseline`), so the draft cell settles to the
				// MIGRATED app. Match it here, else the first post-mount mirror write
				// would diverge from the raw value and trip the overwrite prompt
				// before any edit. Mirrors raw_app's bundle-matching baseline.
				const overlayBaseline = structuredClone($state.snapshot(pendingLoad.value)) as App
				migrateApp(overlayBaseline)
				OtherUserDraftLoad.beginOverlay({
					workspace: $workspaceStore!,
					itemKind: 'app',
					path,
					ownerLabel: pendingLoad.ownerLabel,
					// AppEditor stores the bare (migrated) App in the draft cell.
					loadedValue: overlayBaseline,
					onResetToOwnDraft: async () => {
						await loadApp({ getDraft: true })
						redraw++
					}
				})
			}
		}
		// Pin the fork base for the stale-draft check: when seeding a draft from the
		// deployed app (no own draft yet), stamp the deployed head version onto the
		// draft value. An existing own draft already carries it (preserved by the
		// value swap above). `parent_version` is in DRAFT_COMPARE_IGNORED_FIELDS, so it
		// never trips the autosave no-op / "unsaved changes" comparison.
		if (!hasOwnDraft && !backendApp.no_deployed && backendApp.value) {
			const versions = (backendApp as { versions?: number[] }).versions
			const head = Array.isArray(versions) ? versions[versions.length - 1] : undefined
			if (head != null) (backendApp.value as App).parent_version = head
		}
		// Assign the fresh response onto `app`. The path-change $effect sets
		// `app = undefined` first, unmounting AppEditor and releasing the UserDraft
		// entry, so the remount starts fresh — no local discard is needed here
		// (and `UserDraft.discard` would POST `value: null`, a spurious autosave).
		app = backendApp
	}

	$effect(() => {
		// Re-run on workspace OR path change so navigating from one app editor
		// to another (e.g. via the workspace picker) reloads the new app.
		const currentPath = page.params.path
		if ($workspaceStore && currentPath !== undefined) {
			untrack(() => {
				// Clear the app so AppEditor unmounts; it will remount once loadApp
				// completes with fresh data, re-initializing its internal stores.
				app = undefined
				path = currentPath
				loadApp()
			})
		}
	})

	async function reloadDeployed() {
		UserDraft.remove('app', path)
		await loadApp({ getDraft: false })
		redraw++
	}

	async function restoreDeployed() {
		if (!savedApp || !$workspaceStore) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'app',
			path,
			onResetToDeployed: reloadDeployed
		})
	}

	let diffDrawer: DiffDrawer | undefined = $state()

	function onRestore(restoredApp: any) {
		sendUserToast('App restored from previous deployment')
		// Drop the stale pre-restore autosave. The remounted AppEditor seeds its
		// state from `appDraftHandle.draft ?? app`, so without this it keeps showing
		// the old draft instead of the restored version. Same reason `reloadDeployed`
		// removes the draft before remounting.
		UserDraft.remove('app', path)
		app = restoredApp
		// Re-pin the stale-draft fork base to the current head. A restored value
		// carries the `parent_version` baked in when that older version was deployed,
		// which would make the deploy guard (`compareVersions`) falsely report "not on
		// latest". Restore targets the live app, so `versions` holds the current head.
		const versions = (app as { versions?: number[] } | undefined)?.versions
		const head = Array.isArray(versions) ? versions[versions.length - 1] : undefined
		if (head != null && app?.value) (app.value as App).parent_version = head
		const app_ = structuredClone(stateSnapshot(app!))
		savedApp = {
			summary: app_.summary,
			value: app_.value as App,
			path: app_.path,
			policy: app_.policy,
			custom_path: app_.custom_path,
			labels: app_.labels
		}
		redraw++
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
			itemKind: 'app',
			path
		})}
	onDiscardChanges={() =>
		UserDraftDbSyncer.dropPending({
			workspace: $workspaceStore ?? '',
			itemKind: 'app',
			path
		})}
/>
<DraftEditorModals
	workspace={$workspaceStore ?? ''}
	itemKind="app"
	{path}
	{otherDraftsUsers}
	draftOnly={isNewApp}
	hasOwnDraft={loadedFromDraft}
	onLoadFromServer={async () => {
		// AppEditor's `stateApp` is captured at mount and ignores prop changes,
		// so `redraw++` remounts it against the fresh `app`.
		await loadApp()
		redraw++
	}}
	getLocalDraft={() => app?.value}
	bind:othersModalOpen
	{draftSavedAt}
	{deployedAt}
	{draftBaseVersion}
	{deployedHeadVersion}
	onLoadLatestDeploy={async () => {
		if (!$workspaceStore) return
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'app',
			path,
			onResetToDeployed: reloadDeployed
		})
	}}
/>

{#key redraw}
	{#if app}
		<div class="h-screen">
			<AppEditor
				bind:this={appEditor}
				{fromHub}
				onSavedNewAppPath={(url) => {
					goto(`/apps/edit/${url}`)
					if (app) {
						app.path = url
					}
				}}
				{onRestore}
				summary={app.summary}
				app={app.value}
				labels={app.labels}
				{deployedBaseline}
				newPath={app.value?.draft_path ?? app.path}
				path={page.params.path ?? ''}
				policy={app.policy}
				bind:savedApp
				{diffDrawer}
				version={app.versions ? app.versions[app.versions.length - 1] : undefined}
				newApp={isNewApp}
				replaceStateFn={(path) => replaceState(path, page.state)}
				gotoFn={(path, opt) => goto(path, opt)}
				onResetToDeployed={reloadDeployed}
				{loadedFromDraft}
				othersDraftsCount={otherDraftsUsers.length}
				onOpenOthersDrafts={() => (othersModalOpen = true)}
			/>
		</div>
	{/if}
{/key}
