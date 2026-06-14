<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, type AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { replaceState } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { App } from '$lib/components/apps/types'
	import DraftEditorModals from '$lib/components/common/confirmationModal/DraftEditorModals.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { type OtherDraftUser } from '$lib/components/common/confirmationModal/OtherUsersDraftsModal.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import { emptyApp } from '$lib/components/apps/editor/appUtils'
	import { importStore } from '$lib/components/apps/store'
	import { tick, untrack } from 'svelte'
	import { page } from '$app/state'
	import { UserDraft } from '$lib/userDraft.svelte'
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
		  }
		| undefined = $state(undefined)
	let redraw = $state(0)
	let path = page.params.path ?? ''
	/** No deployed app at the URL path. Drives the editor's deploy:
	 * `createApp` vs `updateApp`. Flips false once a deploy lands here. */
	let isNewApp = $state(false)
	let otherDraftsUsers = $state<OtherDraftUser[]>([])
	let loadedFromDraft = $state(false)
	let othersModalOpen = $state(false)
	let draftSavedAt = $state<string | undefined>(undefined)
	let deployedAt = $state<string | undefined>(undefined)

	/** Increments per `loadApp` call. Stale loads (e.g. when picker
	 * navigation races a draft-discard reload) bail at the next checkpoint
	 * after their captured token no longer matches. */
	let loadAppToken = 0
	async function loadApp(opts: { getDraft?: boolean } = {}): Promise<void> {
		const getDraft = opts.getDraft ?? true
		const tok = ++loadAppToken
		// `?new_draft=true` (from `/apps/add`'s redirect): a fresh, never-saved
		// `draft_{uuid}` path. Skip the backend fetch (would 404) and seed empty
		// with `path = ''` for the friendly auto-name. See /scripts/edit's loader.
		if (page.url.searchParams.get('new_draft') === 'true') {
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
			// Capture every seeding param BEFORE stripping the URL flag.
			const templatePath = page.url.searchParams.get('template')
			const templateId = page.url.searchParams.get('template_id')
			const hubId = page.url.searchParams.get('hub')
			// Explicit path seed: the fork-a-draft handoff re-homes the source
			// path into the forker's namespace and passes it here.
			const pathParam = page.url.searchParams.get('seed_path')
			const url = new URL(window.location.href)
			url.searchParams.delete('new_draft')
			window.history.replaceState(window.history.state, '', url.toString())
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
		let backendApp = await AppService.getAppByPath({
			path: page.params.path ?? '',
			workspace: $workspaceStore!,
			getDraft
		})
		if (tok !== loadAppToken) return
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
				summary: '',
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
			backendApp = { ...backendApp, value: savedDraftApp } as typeof backendApp
		}
		if (backendApp.is_draft) {
			loadedFromDraft = true
		}
		// Pass both timestamps for DraftEditorModals' staleness compare: `created_at`
		// is the deploy time, `draft_saved_at` the draft's. Skip `deployedAt` when
		// `no_deployed` — no baseline to be older than.
		draftSavedAt = backendApp.draft_saved_at as string | undefined
		deployedAt = backendApp.no_deployed ? undefined : (backendApp.created_at as string | undefined)
		const backendApp_ = structuredClone(stateSnapshot(backendApp))
		savedApp = {
			summary: backendApp_.summary,
			value: backendApp_.value as App,
			path: backendApp_.path,
			policy: backendApp_.policy,
			custom_path: backendApp_.custom_path
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

	async function restoreDeployed() {
		if (!savedApp || !$workspaceStore) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		goto(`/apps/edit/${savedApp.path}`)
		// stopSync-bracketed delete + getDraft:false reload; a bare discard +
		// `loadApp()` loses the race. See /scripts/edit's restoreDeployed.
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'app',
			path,
			onResetToDeployed: async () => {
				UserDraft.remove('app', path)
				await loadApp({ getDraft: false })
				redraw++
			}
		})
	}

	let diffDrawer: DiffDrawer | undefined = $state()

	function onRestore(ev: any) {
		sendUserToast('App restored from previous deployment')
		app = ev.detail
		const app_ = structuredClone(stateSnapshot(app!))
		savedApp = {
			summary: app_.summary,
			value: app_.value as App,
			path: app_.path,
			policy: app_.policy,
			custom_path: app_.custom_path
		}
		redraw++
	}
</script>

<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
<DraftEditorModals
	workspace={$workspaceStore ?? ''}
	itemKind="app"
	{path}
	{otherDraftsUsers}
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
	onLoadLatestDeploy={async () => {
		// stopSync-bracketed; see /scripts/edit's restoreDeployed for the race.
		if (!$workspaceStore) return
		await runResetToDeployed({
			workspace: $workspaceStore,
			itemKind: 'app',
			path,
			onResetToDeployed: async () => {
				UserDraft.remove('app', path)
				await loadApp({ getDraft: false })
				redraw++
			}
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
				on:restore={onRestore}
				summary={app.summary}
				app={app.value}
				newPath={app.value?.draft_path ?? app.path}
				path={page.params.path ?? ''}
				policy={app.policy}
				bind:savedApp
				{diffDrawer}
				version={app.versions ? app.versions[app.versions.length - 1] : undefined}
				newApp={isNewApp}
				replaceStateFn={(path) => replaceState(path, page.state)}
				gotoFn={(path, opt) => goto(path, opt)}
				onResetToDeployed={async () => {
					UserDraft.remove('app', path)
					await loadApp({ getDraft: false })
					redraw++
				}}
				{loadedFromDraft}
				othersDraftsCount={otherDraftsUsers.length}
				onOpenOthersDrafts={() => (othersModalOpen = true)}
			/>
		</div>
	{/if}
{/key}
