<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { isMac, userPathPrefix } from '$lib/utils'
	import { editPathFor } from '$lib/components/workspacePicker'
	import { invalidateWorkspacePaths } from '$lib/components/PathNameAutocomplete.svelte'

	import { AppService, type Policy } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import OpenInSessionButton from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { discardDraftAfterDeploy } from '$lib/userDraftToast'
	import { rawAppToHubUrl } from '$lib/hub'
	import {
		enterpriseLicense,
		hubBaseUrlStore,
		userStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import YAML from 'yaml'
	import {
		Bug,
		DiffIcon,
		Download,
		EllipsisVertical,
		FileJson,
		Globe,
		History,
		PanelLeft,
		PanelLeftClose,
		Redo,
		Save,
		Undo,
		WandSparkles
	} from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { orderedJsonStringify, type Value, replaceFalseWithUndefined } from '../../utils'
	import { random_adj } from '$lib/components/random_positive_adjetive'

	// import {  allItems, toStatic } from '../apps/editor/settingsPanel/utils'
	import AppExportButton from '../apps/editor/AppExportButton.svelte'

	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from '../apps/editor/DeploymentHistory.svelte'
	import Awareness from '$lib/components/Awareness.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'

	import EditorHeader from '$lib/components/EditorHeader.svelte'
	import AutosaveIndicator from '$lib/components/AutosaveIndicator.svelte'
	import { goto } from '$app/navigation'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import AppJobsDrawer from '../apps/editor/AppJobsDrawer.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import AppEditorHeaderDeploy from '../apps/editor/AppEditorHeaderDeploy.svelte'
	import type { Runnable } from './RawAppInlineScriptRunnable.svelte'
	import { updateRawAppPolicy } from './rawAppPolicy'
	import { aiChatManager } from '../copilot/chat/AIChatManager.svelte'
	import { getContext } from 'svelte'

	// Forward-looking hook for the upcoming session-pane feature: that PR will
	// `setContext('aiChatManager', ...)` from the session wrapper so this editor
	// can detect it and hide its own AI button. On main today nothing sets the
	// context, so `inSessionPane` is always false and the AI button renders
	// normally — keep the check to avoid re-touching this file when the
	// session-pane PR lands. Untyped getContext to avoid coupling to the
	// AIChatManager class export (which lives on the chat-visuals PR).
	const inSessionPane = !!getContext('aiChatManager')
	// In a session pane the editor does NOT own the localStorage draft — the
	// session runtime does, keyed by the session's (fork) workspace. So the
	// `if (!inSessionPane) UserDraft.remove(...)` guards below skip the editor's
	// own removal (it would target the wrong, main-`$workspaceStore` key). The
	// session-side equivalent is RawAppEditorView's `onDeploy` →
	// `runtime.syncPreviewWithDeployed`, which discards the fork draft + reloads
	// the preview to the deployed version.
	import { AIBtnClasses } from '../copilot/chat/AIButtonStyle'
	import { stripRawAppDiffNoise } from './utils'
	import type { RawAppData } from './dataTableRefUtils'
	import { buildForkEditUrl, editInForkAllowed, editInForkLabel } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'

	// async function hash(message) {
	// 	try {
	// 		const msgUint8 = new TextEncoder().encode(message) // encode as (utf-8) Uint8Array
	// 		const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8) // hash the message
	// 		const hashArray = Array.from(new Uint8Array(hashBuffer)) // convert buffer to byte array
	// 		const hashHex = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
	// 		return hashHex
	// 	} catch {
	// 		//subtle not available, trying pure js
	// 		const hash = new Sha256()
	// 		hash.update(message ?? '')
	// 		const result = Array.from(await hash.digest())
	// 		const hex = result.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
	// 		return hex
	// 	}

	interface Props {
		// }
		summary: string
		policy: Policy
		diffDrawer?: DiffDrawer | undefined
		savedApp?:
			| {
					value: any
					path: string
					summary: string
					policy: any
					custom_path?: string
					labels?: string[]
					/** No deployed counterpart exists (draft-only); disables Diff. */
					no_deployed?: boolean
			  }
			| undefined
		version?: number | undefined
		newApp: boolean
		newPath?: string
		/** Initial labels for the app, threaded from the loaded app data. */
		labels?: string[]
		appPath: string
		runnables: Record<string, Runnable>
		files: Record<string, string> | undefined
		/** Data configuration including tables and creation policy */
		data: RawAppData
		jobs: string[]
		jobsById: Record<string, any>
		getBundle: () => Promise<{
			js: string
			css: string
		}>
		canUndo?: boolean
		canRedo?: boolean
		onUndo?: () => void
		onRedo?: () => void
		onOpenYamlEditor?: () => void
		sidebarCollapsed?: boolean
		onToggleSidebar?: () => void
		onNavigate?: (item: import('$lib/components/workspacePicker').WorkspaceItem) => void
		liveEditorDraftStoragePath?: string
		/** Indicator-only overrides for the sessions preview: the AutosaveIndicator
		 *  watches the session's (workspace, path) so it renders + animates on the
		 *  key SessionEditorTarget saves under. Undefined on the full-page editor →
		 *  falls back to `$workspaceStore`/`liveEditorDraftStoragePath`. */
		autosaveWorkspace?: string
		autosavePath?: string
		// Fired after a successful deploy; lets the session preview reload.
		onDeploy?: (e: { path: string }) => void
		/** Surfaces the user-typed path (`newEditedPath`) up to the route
		 *  when (and only when) it differs from the deployed/seeded
		 *  `savedApp.path`. The route writes it into the autosaved raw-app
		 *  draft as `draft_path` so the home-page row can render the
		 *  friendly name instead of the URL's autogenerated draft slot. */
		pendingDraftPath?: string | undefined
		// Threaded to the `AutosaveIndicator` popover so its "Reset to
		// deployed" button can do the same thing the load-time toast offers.
		onResetToDeployed?: () => void | Promise<void>
		// See ScriptBuilderProps — same semantics for the raw-app editor's
		// indicator.
		loadedFromDraft?: boolean
		othersDraftsCount?: number
		onOpenOthersDrafts?: () => void
		// Restoring an older deployment from the history drawer. A callback prop
		// (not `on:restore` forwarding): forwarding a `createEventDispatcher`
		// event up through these runes-mode components silently drops it.
		onRestore?: (restoredApp: any) => void
		// Deploy created the app at a new path; the page navigates to it. Callback
		// prop for the same reason as `onRestore` — `on:savedNewAppPath` forwarding
		// through these runes-mode components is dropped.
		onSavedNewAppPath?: (path: string) => void
	}

	let {
		summary = $bindable(),
		policy = $bindable(),
		diffDrawer = undefined,
		savedApp = $bindable(undefined),
		version = $bindable(undefined),
		newApp,
		newPath = '',
		labels: initialLabels = undefined,
		appPath,
		runnables,
		data,
		files,
		jobs = $bindable(),
		jobsById = $bindable(),
		getBundle,
		canUndo = false,
		canRedo = false,
		onUndo = undefined,
		onRedo = undefined,
		onOpenYamlEditor = undefined,
		sidebarCollapsed = false,
		onToggleSidebar = undefined,
		onNavigate = undefined,
		liveEditorDraftStoragePath = undefined,
		autosaveWorkspace = undefined,
		autosavePath = undefined,
		onDeploy = undefined,
		pendingDraftPath = $bindable(undefined),
		onResetToDeployed,
		loadedFromDraft = false,
		othersDraftsCount = 0,
		onOpenOthersDrafts,
		onRestore,
		onSavedNewAppPath
	}: Props = $props()

	// Set by the on-behalf-of selector when the publisher picks a user other than
	// themselves. Forwarded as `preserve_on_behalf_of` so the backend keeps the
	// policy's on_behalf_of instead of resetting it to the deploying user.
	let preserveOnBehalfOf = $state(false)

	// The AutosaveIndicator watches these; in the sessions preview they're the
	// session's (workspace, path), else the full-page editor's own values.
	const indicatorWorkspace = $derived(autosaveWorkspace ?? $workspaceStore)
	const indicatorPath = $derived(autosavePath ?? liveEditorDraftStoragePath)

	$effect(() => {
		const typed = newEditedPath
		const baseline = savedApp?.path ?? ''
		untrack(() => {
			pendingDraftPath = typed && typed !== baseline ? typed : undefined
		})
	})

	// `newApp` is true both for a brand-new app AND (in the session preview) for a
	// draft-only one that already has a real path — so prefer the real `appPath`,
	// but NOT a `draft_{uuid}` storage placeholder (a brand-new app is parked at
	// `u/{user}/draft_{uuid}`). A real named path is kept (else its breadcrumb shows
	// a random name and deploy createApps under it); a placeholder still falls
	// through to the friendly generated suggestion.
	let newEditedPath = $state(
		untrack(() => {
			const realAppPath = appPath && !appPath.split('/').pop()?.startsWith('draft_') ? appPath : ''
			return newApp
				? newPath || realAppPath || userPathPrefix($userStore?.username) + random_adj() + '_app'
				: newPath || appPath || ''
		})
	)

	$effect(() => {
		if (liveEditorDraftStoragePath === undefined || !$workspaceStore) return
		const workspace = $workspaceStore
		UserDraft.setLiveEditorDraft({
			workspace,
			itemKind: 'raw_app',
			storagePath: liveEditorDraftStoragePath,
			effectivePath: newEditedPath || appPath || savedApp?.path
		})
		return () =>
			UserDraft.clearLiveEditorDraft('raw_app', {
				workspace,
				storagePath: liveEditorDraftStoragePath
			})
	})

	let deployedValue: Value | undefined = $state(undefined) // Value to diff against
	let deployedBy: string | undefined = $state(undefined) // Author
	let confirmCallback: () => void = $state(() => {}) // What happens when user clicks `override` in warning
	let open: boolean = $state(false) // Is confirmation modal open

	// const { app, summary, appPath, jobs, jobsById, staticExporter } = getContext('AppViewerContext')

	const loading = $state({
		publish: false,
		save: false
	})

	let pathError: string = $state('')
	let appExport = $state() as AppExportButton | undefined

	let saveDrawerOpen = $state(false)
	let historyBrowserDrawerOpen = $state(false)
	let publishToHubDrawerOpen = $state(false)
	let publishingToHub = $state(false)
	let deploymentMsg: string | undefined = $state(undefined)

	// Top-bar responsive collapse — container width, not viewport.
	let topbarWidth = $state(0)
	const compactTopbar = $derived(topbarWidth > 0 && topbarWidth < 720)

	async function publishToHub() {
		if (!app) return
		publishingToHub = true
		try {
			const { default: JSZip } = await import('jszip')
			const { js, css } = await getBundle()
			const zip = new JSZip()
			zip.file('app.yaml', YAML.stringify(app))
			zip.file('bundle.js', js)
			zip.file('bundle.css', css)
			const blob = await zip.generateAsync({ type: 'blob' })

			// Download the zip
			const url = window.URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `${(appPath || 'raw-app').replaceAll('/', '__')}.zip`
			a.click()
			setTimeout(() => URL.revokeObjectURL(url), 100)

			// Open hub page
			const hubUrl = rawAppToHubUrl(
				$hubBaseUrlStore,
				summary || appPath.split('/').pop()?.replace('_', ' ') || 'my raw app'
			)
			window.open(hubUrl.toString(), '_blank')
		} finally {
			publishingToHub = false
		}
	}

	function closeSaveDrawer() {
		saveDrawerOpen = false
	}

	async function computeTriggerables() {
		policy = await updateRawAppPolicy(runnables, policy)
	}

	async function createApp(path: string) {
		if (!app) {
			sendUserToast(`App hasn't been loaded yet`, true)
			return
		}
		if (!policy.execution_mode) {
			policy.execution_mode = 'publisher'
		}
		await computeTriggerables()
		try {
			const { js, css } = await getBundle()
			await AppService.createAppRaw({
				workspace: $workspaceStore!,
				formData: {
					app: {
						value: app,
						path,
						summary: summary,
						policy,
						deployment_message: deploymentMsg,
						custom_path: customPath,
						preserve_on_behalf_of: preserveOnBehalfOf || undefined,
						labels
					},
					js,
					css
				}
			})
			// New path now exists server-side — drop the autocomplete cache so
			// it shows up immediately instead of after the 60s TTL.
			invalidateWorkspacePaths($workspaceStore!)
			savedApp = {
				summary: summary,
				value: structuredClone(stateSnapshot(app)),
				path: path,
				policy: policy,
				custom_path: customPath,
				labels: $state.snapshot(labels)
			}
			closeSaveDrawer()
			sendUserToast('App deployed successfully')
			// Canonical autosave key (the URL slot `appPath`), NOT the
			// just-typed deploy `path` — for a draft-only app they differ
			// (`u/{user}/draft_{uuid}` vs the chosen path), so removing at
			// `path` orphaned the real draft row. Bracketed + flushed:
			// RawAppEditor stays mounted through the post-deploy navigation
			// and its mirror would otherwise displace the queued delete.
			if (!inSessionPane && $workspaceStore) {
				discardDraftAfterDeploy({
					workspace: $workspaceStore,
					itemKind: 'raw_app',
					path: appPath
				})
			}
			onSavedNewAppPath?.(path)
			onDeploy?.({ path })
		} catch (e) {
			sendUserToast(`Error creating app: ${e.body ?? e.message}`, true)
		}
	}

	async function handleUpdateApp(npath: string) {
		// We have to make sure there is no updates when we clicked the button
		await compareVersions()

		if (onLatest) {
			// Handle directly
			await updateApp(npath)
		} else {
			// There is onLatest, but we need more information while deploying
			// We need it to show diff
			// Handle through confirmation modal
			await syncWithDeployed()
			if (
				deployedValue &&
				savedApp &&
				app &&
				orderedJsonStringify(deployedValue) ===
					orderedJsonStringify(replaceFalseWithUndefined(currentDiffValue))
			) {
				await updateApp(npath)
			} else {
				confirmCallback = async () => {
					open = false
					await updateApp(npath)
				}
				// Open confirmation modal
				open = true
			}
		}
	}

	async function syncWithDeployed() {
		const deployedApp = await AppService.getAppByPath({
			workspace: $workspaceStore!,
			path: appPath!,
			withStarredInfo: true
		})

		deployedBy = deployedApp.created_by

		// Normalize away post-deploy noise (see stripRawAppDiffNoise) so the
		// diff/comparison only reflects what the editor actually changed.
		deployedValue = replaceFalseWithUndefined(stripRawAppDiffNoise(deployedApp))
	}

	async function openDiffDrawer() {
		if (!savedApp) {
			return
		}

		// deployedValue should be syncronized when we open Diff
		await syncWithDeployed()

		diffDrawer?.openDrawer()
		diffDrawer?.setDiff({
			mode: 'normal',
			deployed: deployedValue ?? stripRawAppDiffNoise(savedApp),
			current: currentDiffValue
		})
	}

	async function updateApp(npath: string) {
		if (!app) {
			sendUserToast(`App hasn't been loaded yet`, true)
			return
		}
		const { js, css } = await getBundle()
		await computeTriggerables()
		if (!policy.execution_mode) {
			policy.execution_mode = 'publisher'
		}
		await AppService.updateAppRaw({
			workspace: $workspaceStore!,
			path: appPath!,
			formData: {
				app: {
					value: app!,
					summary: summary,
					policy,
					path: npath,
					deployment_message: deploymentMsg,
					preserve_on_behalf_of: preserveOnBehalfOf || undefined,
					// custom_path requires admin so to accept update without it, we need to send as undefined when non-admin (when undefined, it will be ignored)
					// it also means that customPath needs to be set to '' instead of undefined to unset it (when admin)
					custom_path:
						$userStore?.is_admin || $userStore?.is_super_admin ? (customPath ?? '') : undefined,
					labels
				},
				js,
				css
			}
		})
		invalidateWorkspacePaths($workspaceStore!)
		savedApp = {
			summary: summary,
			value: structuredClone(stateSnapshot(app)),
			path: npath,
			policy,
			custom_path: customPath,
			labels: $state.snapshot(labels)
		}
		const appHistory = await AppService.getAppHistoryByPath({
			workspace: $workspaceStore!,
			path: npath
		})
		version = appHistory[0]?.version

		closeSaveDrawer()
		sendUserToast('App deployed successfully')
		// Bracketed + flushed (see createApp).
		if (!inSessionPane && $workspaceStore) {
			discardDraftAfterDeploy({
				workspace: $workspaceStore,
				itemKind: 'raw_app',
				path: appPath
			})
		}
		if (appPath !== npath) {
			onSavedNewAppPath?.(npath)
		}
		onDeploy?.({ path: npath })
	}

	async function setPublishState(message?: string) {
		await computeTriggerables()
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: appPath,
			requestBody: { policy }
		})
		if (message) {
			sendUserToast(message)
		} else if (policy.execution_mode == 'anonymous') {
			sendUserToast('App require no login to be accessed')
		} else {
			sendUserToast('App require login and read-access')
		}
	}

	async function save() {
		saveDrawerOpen = true
		return
	}

	let onLatest = $state(true)
	async function compareVersions() {
		if (version === undefined) {
			return
		}
		try {
			const appVersion = await AppService.getAppLatestVersion({
				workspace: $workspaceStore!,
				path: appPath
			})
			onLatest = appVersion?.version === undefined || version === appVersion?.version
		} catch (e) {
			console.error('Error comparing versions', e)
			onLatest = true
		}
	}

	const mod = isMac() ? '⌘' : 'Ctrl+'

	let moreItems = $derived([
		...(compactTopbar
			? [
					{
						displayName: `Jobs (${jobs?.length > 99 ? '99+' : (jobs?.length ?? 0)})`,
						icon: Bug,
						action: () => (jobsDrawerOpen = true),
						separatorBottom: true
					}
				]
			: []),
		{
			displayName: 'Undo',
			icon: Undo,
			action: () => onUndo?.(),
			disabled: !canUndo,
			shortcut: `${mod}Z`
		},
		{
			displayName: 'Redo',
			icon: Redo,
			action: () => onRedo?.(),
			disabled: !canRedo,
			shortcut: `${mod}⇧Z`,
			separatorBottom: true
		},
		{
			displayName: 'Deployment history',
			icon: History,
			action: () => {
				historyBrowserDrawerOpen = true
			},
			disabled: !savedApp
		},
		{
			displayName: 'Export',
			icon: FileJson,
			action: () => {
				appExport?.open(app)
			}
		},
		{
			displayName: 'Edit in YAML',
			icon: FileJson,
			action: () => onOpenYamlEditor?.()
		},
		{
			displayName: 'Publish to Hub',
			icon: Globe,
			action: () => {
				publishToHubDrawerOpen = true
			}
		}
	])

	let customPath = $state(savedApp?.custom_path)
	let customPathError = $state('')
	let labels = $state(untrack(() => initialLabels))

	let jobsDrawerOpen = $state(false)

	let app = $derived(files ? { runnables: runnables, files, data } : undefined)

	// Editor-side value for diffing/comparison against the deployed app, with the
	// same noise stripped as the deployed side (see stripRawAppDiffNoise).
	let currentDiffValue = $derived(
		stripRawAppDiffNoise({
			summary: summary,
			value: app,
			path: newEditedPath || savedApp?.path,
			policy,
			custom_path: customPath,
			labels
		})
	)

	$effect(() => {
		saveDrawerOpen && compareVersions()
	})
</script>

<DeployOverrideConfirmationModal
	{deployedBy}
	{confirmCallback}
	bind:open
	{diffDrawer}
	bind:deployedValue
	currentValue={currentDiffValue}
/>

<Drawer bind:open={saveDrawerOpen} size="800px">
	<DrawerContent title="Deploy" on:close={() => closeSaveDrawer()}>
		{#snippet actions()}
			<div class="flex flex-row gap-2">
				<Button
					variant="default"
					disabled={!savedApp || newApp}
					on:click={async () => {
						if (!savedApp || newApp) {
							return
						}
						// deployedValue should be syncronized when we open Diff
						await syncWithDeployed()

						saveDrawerOpen = false
						diffDrawer?.openDrawer()
						diffDrawer?.setDiff({
							mode: 'normal',
							deployed: deployedValue ?? stripRawAppDiffNoise(savedApp),
							current: currentDiffValue,
							button: {
								text: 'Looks good, deploy',
								onClick: () => {
									if (newApp || appPath == '') {
										createApp(newEditedPath)
									} else {
										handleUpdateApp(newEditedPath)
									}
								}
							}
						})
					}}
				>
					<div class="flex flex-row gap-2 items-center">
						<DiffIcon size={14} />
						Diff
					</div>
				</Button>
				<Button
					variant="accent"
					unifiedSize="md"
					startIcon={{ icon: Save }}
					disabled={pathError != '' || customPathError != '' || app == undefined}
					on:click={() => {
						if (newApp || appPath == '') {
							createApp(newEditedPath)
						} else {
							handleUpdateApp(newEditedPath)
						}
					}}
				>
					Deploy
				</Button>
			</div>
		{/snippet}

		<AppEditorHeaderDeploy
			{newPath}
			{newApp}
			{policy}
			{setPublishState}
			{appPath}
			{onLatest}
			{savedApp}
			rawApp
			bind:summary
			bind:customPath
			bind:deploymentMsg
			bind:customPathError
			bind:pathError
			bind:newEditedPath
			bind:preserveOnBehalfOf
			bind:labels
		/>
	</DrawerContent>
</Drawer>

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory on:restore={(e) => onRestore?.(e.detail)} {appPath} />
	</DrawerContent>
</Drawer>

<Drawer bind:open={publishToHubDrawerOpen} size="600px">
	<DrawerContent title="Publish to Hub" on:close={() => (publishToHubDrawerOpen = false)}>
		{#snippet actions()}
			<Button
				loading={publishingToHub}
				disabled={!app}
				on:click={publishToHub}
				variant="accent"
				startIcon={{ icon: Download }}
			>
				Download & open hub
			</Button>
		{/snippet}
		<div class="flex flex-col gap-4">
			<p class="text-secondary text-sm">
				This will download a zip file containing your raw app bundle and open the Windmill Hub
				submission page.
			</p>
			<div class="text-sm">
				<p class="font-semibold mb-2">The zip file will contain:</p>
				<ul class="list-disc list-inside text-secondary space-y-1">
					<li
						><code class="text-xs bg-surface-secondary px-1 rounded">app.yaml</code> - App configuration</li
					>
					<li
						><code class="text-xs bg-surface-secondary px-1 rounded">bundle.js</code> - JavaScript bundle</li
					>
					<li
						><code class="text-xs bg-surface-secondary px-1 rounded">bundle.css</code> - CSS styles</li
					>
				</ul>
			</div>
		</div>
	</DrawerContent>
</Drawer>

<AppJobsDrawer
	bind:open={jobsDrawerOpen}
	on:clear={() => {
		jobs = []
	}}
	on:clearErrors={() => {
		console.log('todo clear errors')
	}}
	{jobs}
	hasErrors={false}
	{jobsById}
	errorByComponent={{}}
/>

<div
	bind:clientWidth={topbarWidth}
	class="flex flex-row justify-between gap-2 gap-y-2 px-2 items-center overflow-y-visible overflow-x-auto max-h-12 h-12 shrink-0"
>
	<!-- Identity block: shrinks/truncates first so the cloud indicator and the
	     action buttons stay visible. Without min-w-0 the breadcrumb + summary
	     overflow this box on narrow widths and get overlapped by the (formerly
	     un-pinned) action group, hiding the autosave cloud. -->
	<div class="flex flex-row gap-2 items-center min-w-0">
		{#if onToggleSidebar}
			<Button
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{ icon: sidebarCollapsed ? PanelLeft : PanelLeftClose }}
				title={`${sidebarCollapsed ? 'Expand' : 'Collapse'} file sidebar (${isMac() ? '⌘' : 'Ctrl+'}B)`}
				on:click={() => onToggleSidebar?.()}
			/>
		{/if}
		<div class="min-w-0 overflow-hidden">
			<EditorHeader
				bind:summary
				bind:path={newEditedPath}
				savedPath={appPath || newPath || undefined}
				kind="app"
				raw_app
				onNavigate={(item) => (onNavigate ? onNavigate(item) : goto(editPathFor(item)))}
			/>
		</div>
		{#if indicatorWorkspace && indicatorPath !== undefined}
			<AutosaveIndicator
				workspace={indicatorWorkspace}
				itemKind="raw_app"
				path={indicatorPath}
				draftOnly={newApp}
				{onResetToDeployed}
				{loadedFromDraft}
				{othersDraftsCount}
				{onOpenOthersDrafts}
			/>
		{/if}
	</div>

	{#if $enterpriseLicense && appPath != ''}
		<div class="shrink-0">
			<Awareness />
		</div>
	{/if}
	<div class="flex flex-row gap-2 justify-end items-center overflow-visible shrink-0">
		<DropdownV2 items={moreItems} class="h-auto">
			{#snippet buttonReplacement()}
				<Button
					nonCaptureEvent
					unifiedSize="md"
					variant="subtle"
					startIcon={{ icon: EllipsisVertical }}
					iconOnly
				></Button>
			{/snippet}
		</DropdownV2>

		<!-- A disabled <button> fires no pointer events, so a title/tooltip on it
		     never shows on hover. pointer-events-none on the button lets the hover
		     reach this titled wrapper instead. -->
		<div
			title={newApp || savedApp?.no_deployed === true
				? 'Deploy this app once to compare against the deployed version'
				: 'Diff'}
			class={!savedApp || newApp || savedApp?.no_deployed === true
				? 'flex cursor-not-allowed'
				: 'flex'}
		>
			<Button
				variant="default"
				unifiedSize="md"
				on:click={() => openDiffDrawer()}
				disabled={!savedApp || newApp || savedApp?.no_deployed === true}
				btnClasses={!savedApp || newApp || savedApp?.no_deployed === true
					? 'pointer-events-none'
					: undefined}
				iconOnly={compactTopbar}
				title={newApp || savedApp?.no_deployed === true
					? 'Deploy this app once to compare against the deployed version'
					: 'Diff'}
				startIcon={{ icon: DiffIcon }}
			>
				Diff
			</Button>
		</div>

		<div class="{compactTopbar ? 'hidden' : 'hidden md:inline'} relative overflow-visible">
			<Button
				on:click={() => {
					jobsDrawerOpen = true
				}}
				color="light"
				unifiedSize="md"
				variant="default"
				btnClasses="relative"
			>
				<div class="flex flex-row gap-1 items-center">
					<Bug size={14} />
					<div>Jobs</div>

					<div class="text-2xs text-primary"
						>({jobs?.length > 99 ? '99+' : (jobs?.length ?? 0)})</div
					>
				</div>
			</Button>
		</div>
		<AppExportButton bind:this={appExport} />
		<OpenInSessionButton
			source={appPath
				? {
						target: { kind: 'raw_app', path: appPath },
						workspaceId: $workspaceStore ?? undefined,
						// Flush the autosaved draft so the session preview opens the app
						// exactly as it is in the editor right now.
						beforeOpen: () =>
							indicatorWorkspace && indicatorPath !== undefined
								? UserDraftDbSyncer.flush({
										workspace: indicatorWorkspace,
										itemKind: 'raw_app',
										path: indicatorPath
									})
								: undefined
					}
				: undefined}
		>
			{#snippet fallback()}
				<Button
					unifiedSize="md"
					variant="default"
					onClick={() => aiChatManager.toggleOpen()}
					startIcon={{ icon: WandSparkles }}
					iconOnly
					btnClasses={AIBtnClasses('default')}
				>
					AI
				</Button>
			{/snippet}
		</OpenInSessionButton>
		<Button
			loading={loading.save}
			startIcon={{ icon: Save }}
			on:click={save}
			unifiedSize="md"
			variant="accent"
			dropdownItems={appPath != ''
				? () => [
						{
							label: 'Fork',
							onClick: () => {
								window.open(`/apps/add?template=${appPath}`)
							}
						},
						...(!isCloudHosted() && editInForkAllowed($workspaceStore, $userWorkspaces)
							? [
									{
										label: editInForkLabel($workspaceStore, $userWorkspaces),
										onClick: () => {
											window.open(buildForkEditUrl('raw_app', appPath))
										}
									}
								]
							: [])
					]
				: undefined}
		>
			Deploy
		</Button>
	</div>
</div>
