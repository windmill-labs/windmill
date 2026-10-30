<script lang="ts">
	import { deployAsksOnBehalfOf } from '$lib/components/itemSettings/onBehalfOf'
	import {
		fetchDeployedAccessMode,
		fetchDeployedRunAs,
		setDeployedOnBehalfOf
	} from '$lib/components/apps/editor/appDeploy.svelte'
	import DeployOnBehalfOfDialog from '$lib/components/itemSettings/DeployOnBehalfOfDialog.svelte'
	import { useFolderDefaultPermissionedAs } from '$lib/components/useFolderDefaultPermissionedAs.svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import RawAppRecordSession from '$lib/components/workspaceSettings/RawAppRecordSession.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { isMac, userPathPrefix, emptyString } from '$lib/utils'
	import { editPathFor } from '$lib/components/workspacePicker'
	import { invalidateWorkspacePaths } from '$lib/components/PathNameAutocomplete.svelte'

	import { AppService, type Policy } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import OpenInSessionButton, {
		type OpenInSessionSource
	} from '$lib/components/sessions/OpenInSessionButton.svelte'
	import { discardDraftAfterDeploy } from '$lib/userDraftToast'
	import { enterpriseLicense, userStore, userWorkspaces } from '$lib/stores'
	import {
		Bug,
		DiffIcon,
		EllipsisVertical,
		FileJson,
		Circle,
		Globe,
		History,
		PanelLeft,
		PanelLeftClose,
		Redo,
		Settings,
		Undo,
		UserPlus,
		WandSparkles
	} from 'lucide-svelte'
	import ShareDialog from '$lib/components/itemSettings/ShareDialog.svelte'
	import { showAppLinks } from '$lib/components/itemSettings/AppLinksModal.svelte'
	import FirstDeployDialog, {
		applyFirstDeployMembers
	} from '$lib/components/itemSettings/FirstDeployDialog.svelte'
	import ShareButton from '$lib/components/itemSettings/ShareButton.svelte'
	import { accessModeOf } from '$lib/components/itemSettings/appAccess'
	import { movesToAnotherOwner } from '$lib/components/itemSettings/inheritedAccess'
	import { onDestroy, untrack, type Snippet } from 'svelte'
	import { orderedJsonStringify, type Value, replaceFalseWithUndefined } from '../../utils'
	import { random_adj } from '$lib/components/random_positive_adjetive'

	// import {  allItems, toStatic } from '../apps/editor/settingsPanel/utils'
	import AppExportButton from '../apps/editor/AppExportButton.svelte'

	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from '../apps/editor/DeploymentHistory.svelte'
	import Awareness from '$lib/components/Awareness.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import { VERSION_PAGE_SIZE } from '$lib/components/diff_drawer'

	import EditorHeader from '$lib/components/EditorHeader.svelte'
	import AutosaveIndicator from '$lib/components/AutosaveIndicator.svelte'
	import { goto } from '$app/navigation'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import AppJobsDrawer from '../apps/editor/AppJobsDrawer.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import AppSettingsDrawer from '../apps/editor/AppSettingsDrawer.svelte'
	import DeployButton from '$lib/components/DeployButton.svelte'
	import type { Runnable } from './RawAppInlineScriptRunnable.svelte'
	import { updateRawAppPolicy } from './rawAppPolicy'
	import { withoutSharedPolicySettings } from '../apps/editor/appDraftSettings'
	import { saveRawAppFork } from './forkRawApp.svelte'
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
	// own removal (it would target the wrong, navigation-workspace key). The
	// session-side equivalent is RawAppEditorView's `onDeploy` →
	// `runtime.itemDeployed`, which discards the fork draft + reloads
	// the preview to the deployed version.
	import { AIBtnClasses } from '../copilot/chat/AIButtonStyle'
	import { stripRawAppDiffNoise } from './utils'
	import type { RawAppData } from './dataTableRefUtils'
	import { editInForkAllowed, editInForkLabel, openEditInFork } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'
	import {
		operatingWorkspaceContextEntry,
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import PathEditPopover from '$lib/components/PathEditPopover.svelte'

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)

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
		/** Moves the draft's base to the deployed head and keeps its content;
		 *  offered in the diff drawer while the draft is behind. */
		onTakeLatest?: (head?: string) => void | Promise<void>
		/** The app_version the draft forked from; the drawer offers `onTakeLatest` only
		 *  while it differs from the head on display. */
		draftBaseVersion?: string | undefined
		newApp: boolean
		newPath?: string
		/** Initial labels for the app, threaded from the loaded app data. */
		labels?: string[]
		appPath: string
		/** "Open in AI session" hand-off, owned by the editor (it persists the
		 * draft the session preview loads). Undefined until the app has a path. */
		sessionOpen?: OpenInSessionSource
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
		/** Condensed top bar: smaller (sm) buttons, a shorter bar, and the
		 * EditorHeader's path/breadcrumb row dropped (summary only). Used by the
		 * session preview to save vertical room. */
		condensedHeader?: boolean
		/** True for the route's own editor: its top bar becomes the page header. */
		ownsPageHeader?: boolean
		onNavigate?: (item: import('$lib/components/workspacePicker').WorkspaceItem) => void
		liveEditorDraftStoragePath?: string
		/** Indicator-only overrides for the sessions preview: the AutosaveIndicator
		 *  watches the session's (workspace, path) so it renders + animates on the
		 *  key SessionEditorTarget saves under. Undefined on the full-page editor →
		 *  falls back to the operating workspace/`liveEditorDraftStoragePath`. */
		autosaveWorkspace?: string
		autosavePath?: string
		// Fired after a successful deploy; lets the session preview reload. `version` is
		// what this deploy wrote, for the next draft's fork base; `head` is what is
		// deployed now, with its author and time, and the two differ when another deploy
		// landed beside this one.
		onDeploy?: (e: {
			path: string
			version?: number
			head?: number
			headBy?: string
			headAt?: string
		}) => void
		/** Surfaces the user-typed path (`newEditedPath`) up to the route
		 *  when (and only when) it differs from the deployed/seeded
		 *  `savedApp.path`. The route writes it into the autosaved raw-app
		 *  draft as `draft_path` so the home-page row can render the
		 *  friendly name instead of the URL's autogenerated draft slot. */
		pendingDraftPath?: string | undefined
		/** The labels being edited, which the route writes into the autosaved draft. */
		pendingDraftSettings?: { labels: string[] } | undefined
		// Threaded to the `AutosaveIndicator` popover so its "Reset to
		// deployed" button can do the same thing the load-time toast offers.
		/** `Exit & see details`: leaves the editor for the app's deployed page. A session hosts that
		 *  page itself, so it flips the tab there rather than navigating out of the session. */
		onDetails?: (e: { path: string }) => void
		/** Opens the draft `Fork` saved. Without it, the fork opens in a new browser tab; a
		 *  session opens it as one of its own tabs instead. */
		onOpenFork?: (path: string) => void
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
		onTakeLatest = undefined,
		draftBaseVersion = undefined,
		newApp,
		newPath = '',
		labels: initialLabels = undefined,
		appPath,
		sessionOpen,
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
		condensedHeader = false,
		ownsPageHeader = false,
		onNavigate = undefined,
		liveEditorDraftStoragePath = undefined,
		autosaveWorkspace = undefined,
		autosavePath = undefined,
		onDeploy = undefined,
		pendingDraftPath = $bindable(undefined),
		pendingDraftSettings = $bindable(),
		onDetails,
		onOpenFork,
		onResetToDeployed,
		loadedFromDraft = false,
		othersDraftsCount = 0,
		onOpenOthersDrafts,
		onRestore,
		onSavedNewAppPath
	}: Props = $props()

	// Share and the first-deploy dialog belong to Windmill's own editors: an embedding host can
	// lock the path, which the dialog would let the user change.
	const offersSharing = $derived(ownsPageHeader || inSessionPane)
	const isNewApp = $derived(newApp || appPath == '')
	// Only the app's first deploy takes the folder's default user.
	const folderDefault = useFolderDefaultPermissionedAs(
		() => (isNewApp ? newEditedPath : undefined),
		() => opWorkspace
	)
	let deployDialogOpen = $state(false)
	let deployButton: DeployButton | undefined = $state(undefined)
	let deployedRunAs: Awaited<ReturnType<typeof fetchDeployedRunAs>> = $state()

	// The AutosaveIndicator watches these; in the sessions preview they're the
	// session's (workspace, path), else the full-page editor's own values.
	const opWorkspace = $derived(autosaveWorkspace ?? $operatingWorkspace)
	const indicatorPath = $derived(autosavePath ?? liveEditorDraftStoragePath)

	async function forkApp() {
		if (!opWorkspace || !files) return
		const workspace = opWorkspace
		// Opened before the save: a tab opened after an await is no longer tied to the
		// click, and browsers block it as a popup.
		const tab = onOpenFork ? undefined : window.open('', '_blank')
		const forkPath = await saveRawAppFork(workspace, { files, runnables, data, policy })
		if (!forkPath) {
			tab?.close()
			return
		}
		if (onOpenFork) onOpenFork(forkPath)
		else if (tab) tab.location.href = `/apps_raw/edit/${forkPath}?workspace=${workspace}`
	}

	$effect(() => {
		const typed = newEditedPath
		const baseline = savedApp?.path ?? ''
		const settings = { labels: $state.snapshot(labels) ?? [] }
		untrack(() => {
			pendingDraftPath = typed && typed !== baseline ? typed : undefined
			pendingDraftSettings = settings
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
		if (liveEditorDraftStoragePath === undefined || !opWorkspace) return
		const workspace = opWorkspace
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

	let settingsOpen = $state(false)
	let settingsDrawer: AppSettingsDrawer | undefined = $state()
	let historyBrowserDrawerOpen = $state(false)
	let recordDrawer = $state<Drawer | undefined>(undefined)
	let deploymentMsg: string | undefined = $state(undefined)

	// Top-bar responsive collapse — container width, not viewport.
	let topbarWidth = $state(0)
	// In the page header the bar folds the buttons (`actionFolds` below), once its breadcrumb has
	// given all it can; anywhere else this editor's own top bar is theirs alone.
	const compactTopbar = $derived(
		ownsPageHeader ? pageHeader.actionFold >= 1 : topbarWidth > 0 && topbarWidth < 720
	)
	// In the page header no fold drops a button's label: what does not fit goes into the menu.
	// The second fold keeps the trail, the file sidebar's toggle and Deploy; Diff joins the menu.
	// The docked-chat AI toggle stands down — the sidebar's own Ask AI is the way in at that
	// width — while Jobs and the
	// export are already in the menu, Jobs from `compactTopbar` (true at the second fold too) and
	// the export always.
	const phoneTopbar = $derived(ownsPageHeader && pageHeader.actionFold >= 2)
	// The third fold leaves the autosave indicator as its icon, so the AI session button still fits
	// right before Deploy on a phone.
	const narrowTopbar = $derived(ownsPageHeader && pageHeader.actionFold >= 3)
	// The last fold, for the narrowest phones: the file sidebar's toggle joins the menu, which is
	// then the way back to a collapsed sidebar.
	const sidebarToggleInMenu = $derived(ownsPageHeader && pageHeader.actionFold >= 4)

	// Top-bar button size + bar height. Condensed (session preview) uses the
	// smallest well-supported unified size (`sm`) so the bar is thinner.
	// In the page header the buttons ride a 44px band, the same size the condensed session-pane
	// header uses.
	const headerBtnSize = $derived(condensedHeader || ownsPageHeader ? 'sm' : 'md')

	/** Held by the rename popover while it is open; the band's trail reads it so the path does not
	 *  reflow under the pointer as the user types. */
	let pathSnapshot = $state<string | undefined>(undefined)

	// The actions render under the band, outside the subtree RawAppEditor declared the operating
	// workspace in, so it has to travel with them: OpenInSessionButton asks who the acting user is.
	const headerContexts = new Map<any, any>([operatingWorkspaceContextEntry()])

	// Returns a copy: `policy` is the editor's own object, which the settings drawer's
	// toggles mutate in place. Reassigning it would detach the drawer from the
	// editor, so later toggles would never reach the preview. After a successful save,
	// copy only `triggerables_v2` back so the editor's value matches what was deployed.
	function policyWithTriggerables() {
		return updateRawAppPolicy(runnables, policy)
	}

	/** Resolves to whether the app was created. */
	async function createApp(path: string): Promise<boolean> {
		if (!app) {
			sendUserToast(`App hasn't been loaded yet`, true)
			return false
		}
		if (!policy.execution_mode) {
			policy.execution_mode = 'publisher'
		}
		// Read for the path being created: the first-deploy dialog may have just chosen it.
		setDeployedOnBehalfOf(policy, actingUser, await folderDefault.refetch())
		try {
			const { js, css } = await getBundle()
			const deployedPolicy = await policyWithTriggerables()
			await AppService.createAppRaw({
				workspace: opWorkspace!,
				formData: {
					app: {
						value: app,
						path,
						summary: summary,
						policy: deployedPolicy,
						deployment_message: deploymentMsg,
						custom_path: customPath,
						// The user the deploy dialog confirmed, which the backend keeps only for a
						// deployer who may preserve one.
						preserve_on_behalf_of: true,
						labels
					},
					js,
					css
				}
			})
			policy.triggerables_v2 = deployedPolicy.triggerables_v2
			// New path now exists server-side — drop the autocomplete cache so
			// it shows up immediately instead of after the 60s TTL.
			invalidateWorkspacePaths(opWorkspace!)
			savedApp = {
				summary: summary,
				value: structuredClone(stateSnapshot(app)),
				path: path,
				policy: deployedPolicy,
				custom_path: customPath,
				labels: $state.snapshot(labels)
			}
			sendUserToast('App deployed successfully')
			deployButton?.clearMessage()
			// Canonical autosave key (the URL slot `appPath`), NOT the
			// just-typed deploy `path` — for a draft-only app they differ
			// (`u/{user}/draft_{uuid}` vs the chosen path), so removing at
			// `path` orphaned the real draft row. Bracketed + flushed:
			// RawAppEditor stays mounted through the post-deploy navigation
			// and its mirror would otherwise displace the queued delete.
			if (!inSessionPane && $operatingWorkspace) {
				discardDraftAfterDeploy({
					workspace: $operatingWorkspace,
					itemKind: 'raw_app',
					path: appPath
				})
			}
			onSavedNewAppPath?.(path)
			onDeploy?.({ path })
			return true
		} catch (e) {
			sendUserToast(`Error creating app: ${e.body ?? e.message}`, true)
			return false
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

	async function syncWithDeployed(opening?: number) {
		const deployedApp = await AppService.getAppByPath({
			workspace: opWorkspace!,
			path: appPath!
		})

		// A superseded opening must not write these: the current one would then render
		// and offer Take latest against the older head.
		if (opening != null && !diffDrawer?.ownsOpening(opening)) return
		deployedBy = deployedApp.created_by
		const shownVersions = (deployedApp as { versions?: number[] }).versions
		deployedVersionShown = Array.isArray(shownVersions)
			? shownVersions[shownVersions.length - 1]
			: undefined

		// Normalize away post-deploy noise (see stripRawAppDiffNoise) so the
		// diff/comparison only reflects what the editor actually changed.
		deployedValue = replaceFalseWithUndefined(stripRawAppDiffNoise(deployedApp))
	}

	/** The app_version the payload in `deployedValue` came from, so the picker marks that
	 *  one as head rather than trusting the history's first row. */
	let deployedVersionShown: number | undefined = $state(undefined)

	/** Deployed versions for the diff picker, newest first. Best-effort: losing the
	 *  list costs the picker, not the diff. */
	/** Throws: the drawer says so and lets the reader ask for the same page again. */
	async function fetchVersionPage(page: number) {
		if (!opWorkspace || !appPath) return undefined
		{
			const history = await AppService.getAppHistoryByPath({
				workspace: opWorkspace,
				path: appPath,
				page,
				perPage: VERSION_PAGE_SIZE
			})
			// Head is the version the payload beside this list came from; see FlowBuilder.
			const head = deployedVersionShown ?? history[0]?.version
			// No ordinal: the list arrives a page at a time, so a number counted within one
			// would rename versions as more load.
			return history.map((h) => {
				const detail = [
					h.created_by,
					h.created_at ? new Date(h.created_at).toLocaleString() : undefined,
					h.deployment_msg
				].filter(Boolean)
				const isHead = h.version === head
				return {
					id: String(h.version),
					label: `${h.version}${isHead ? ' · latest' : ''}`,
					subtitle: detail.length ? detail.join(' · ') : undefined,
					isHead
				}
			})
		}
	}

	/** Names the deployed side when the picker has no list to label it with — both are
	 *  read in `syncWithDeployed`, which succeeds independently of the history. */
	function deployedVersionLabel(): string | undefined {
		if (deployedVersionShown == null) return undefined
		return `Deployed ${deployedVersionShown}${deployedBy ? ` by ${deployedBy}` : ''} · latest`
	}

	/** The first page, best-effort: losing it costs the picker, not the diff. */
	async function deployedVersionOptions() {
		try {
			return await fetchVersionPage(1)
		} catch {
			return undefined
		}
	}

	/** Hands the drawer the next page each time the reader asks for one. The page number
	 *  belongs to this item's history, so it lives here — and only moves once a page has
	 *  actually arrived, or a failed request would skip it. */
	function moreVersionsLoader() {
		let loaded = 1
		return async () => {
			const page = await fetchVersionPage(loaded + 1)
			loaded += 1
			return page
		}
	}

	/** The opening this editor claimed last. A path change remounts this editor while the
	 *  drawer stays mounted, so its teardown hands that opening back rather than leaving
	 *  the drawer on the item the user left. */
	let lastOpening: number | undefined = undefined
	onDestroy(() => {
		if (lastOpening != null) diffDrawer?.abandonOpening(lastOpening)
	})

	export async function openDiffDrawer() {
		if (!savedApp) {
			return
		}
		// The fetches below are awaited, so a reopen (or a path change, which remounts
		// this editor but not the drawer) while they run must not have the older one
		// land last. The drawer counts the openings for that reason.
		const opening = diffDrawer?.beginOpening()
		lastOpening = opening
		if (opening == null) return

		// deployedValue should be syncronized when we open Diff
		await syncWithDeployed(opening)

		// Blanking the drawer belongs to the opening that will fill it.
		if (!diffDrawer?.ownsOpening(opening)) return
		diffDrawer.openDrawer(opening)
		const versions = await deployedVersionOptions()
		if (!diffDrawer?.ownsOpening(opening)) return
		diffDrawer.setDiff(
			{
				mode: 'normal',
				deployed: deployedValue ?? stripRawAppDiffNoise(savedApp),
				deployedLabel: deployedVersionLabel(),
				versions,
				loadMoreVersions: moreVersionsLoader(),
				onTakeLatest,
				draftBase: draftBaseVersion,
				deployedHead: deployedVersionShown != null ? String(deployedVersionShown) : undefined,
				loadVersion: async (id) => {
					const v = await AppService.getAppByVersion({ workspace: opWorkspace!, id: Number(id) })
					// Same normalization as `syncWithDeployed`, so switching versions doesn't
					// reintroduce the post-deploy noise the head side already strips.
					return replaceFalseWithUndefined(stripRawAppDiffNoise(v as any))
				},
				current: currentDiffValue,
				aiReview: opWorkspace
					? { workspaceId: opWorkspace, kind: 'raw_app', path: appPath }
					: undefined
			},
			opening
		)
	}

	async function updateApp(npath: string) {
		if (!app) {
			sendUserToast(`App hasn't been loaded yet`, true)
			return
		}
		const { js, css } = await getBundle()
		setDeployedOnBehalfOf(policy, actingUser)
		const deployedPolicy = await policyWithTriggerables()
		const deployed = await AppService.updateAppRaw({
			workspace: opWorkspace!,
			path: appPath!,
			formData: {
				app: {
					value: app!,
					summary: summary,
					// Without the mode, which the Share dialog sets: the server keeps the deployed one,
					// read under the row lock a mode change also takes.
					policy: withoutSharedPolicySettings(deployedPolicy),
					path: npath,
					deployment_message: deploymentMsg,
					preserve_on_behalf_of: true,
					// Absent keeps the custom URL, which the Share dialog sets.
					custom_path: undefined,
					labels
				},
				js,
				css
			}
		})
		policy.triggerables_v2 = deployedPolicy.triggerables_v2
		invalidateWorkspacePaths(opWorkspace!)
		const accessMode = await fetchDeployedAccessMode(opWorkspace!, npath)
		if (accessMode) policy.execution_mode = deployedPolicy.execution_mode = accessMode
		savedApp = {
			summary: summary,
			value: structuredClone(stateSnapshot(app)),
			path: npath,
			policy: deployedPolicy,
			custom_path: customPath,
			labels: $state.snapshot(labels)
		}
		const head = await AppService.getAppLatestVersion({
			workspace: opWorkspace!,
			path: npath
		})
		// The deploy's own answer is the next draft's base; `version` is what is deployed
		// now, and the two differ when another deploy landed beside this one. The route
		// owns this prop and re-pushes `parentVersion ?? head` as soon as `onDeploy` returns.
		version = head?.version

		sendUserToast('App deployed successfully')
		deployButton?.clearMessage()
		// Bracketed + flushed (see createApp).
		if (!inSessionPane && $operatingWorkspace) {
			discardDraftAfterDeploy({
				workspace: $operatingWorkspace,
				itemKind: 'raw_app',
				path: appPath
			})
		}
		if (appPath !== npath) {
			onSavedNewAppPath?.(npath)
		}
		onDeploy?.({
			path: npath,
			version: deployed.version,
			head: version,
			headBy: head?.created_by,
			headAt: head?.created_at
		})
	}

	function openSettings(setting?: string) {
		settingsDrawer?.openAt(setting)
	}

	const neverDeployed = $derived(newApp || appPath == '' || savedApp?.no_deployed === true)
	let shareDialog: ShareDialog | undefined = $state()
	let firstDeployDialog: FirstDeployDialog | undefined = $state()

	function openShare() {
		shareDialog?.openDialog('raw_app', appPath, {
			source: 'editor_header',
			fromEditor: true,
			stagedPath: newEditedPath
		})
	}

	// Chosen in the first-deploy dialog, added once the run-as confirmation has created the app.
	let firstDeployMembers: Record<string, boolean> = {}

	async function deploy(message?: string) {
		if (app == undefined) return
		if (pathError != '') {
			openSettings()
			return
		}
		deploymentMsg = message
		firstDeployMembers = {}
		if (isNewApp && firstDeployDialog) {
			// The dialog also asks who the app runs on behalf of, for the path it chose.
			const choice = await firstDeployDialog.ask({
				kind: 'raw_app',
				summary,
				path: newEditedPath,
				appMode: accessModeOf(policy),
				customPath,
				runAs: {
					email: policy.on_behalf_of_email,
					permissionedAs: policy.on_behalf_of,
					targetWorkspace: opWorkspace ?? ''
				}
			})
			if (!choice) return
			summary = choice.summary
			newEditedPath = choice.path
			policy.execution_mode = choice.appMode
			customPath = choice.customPath
			if (choice.runAs) {
				policy.on_behalf_of_email = choice.runAs.email
				policy.on_behalf_of = choice.runAs.permissionedAs
			}
			deploymentMsg = choice.message
			firstDeployMembers = choice.members
			await deployNow()
			return
		}
		// Compares against the user deployed right now; a failed lookup asks rather than trusting
		// the loaded one.
		deployedRunAs = isNewApp ? undefined : await fetchDeployedRunAs(opWorkspace!, appPath)
		const asksRunAs =
			(!isNewApp && !deployedRunAs) ||
			// Asks while the folder default is loading: the backend would apply it unconfirmed.
			folderDefault.loading ||
			deployAsksOnBehalfOf('app', {
				user: actingUser,
				email: policy.on_behalf_of_email,
				permissionedAs: policy.on_behalf_of,
				savedEmail: deployedRunAs?.on_behalf_of_email,
				savedPermissionedAs: deployedRunAs?.on_behalf_of,
				folderDefault: folderDefault.value
			})
		if (
			shareDialog &&
			movesToAnotherOwner(appPath, newEditedPath) &&
			!(await shareDialog.confirmMove(
				'raw_app',
				appPath,
				newEditedPath,
				asksRunAs ? 'Next' : 'Deploy'
			))
		) {
			return
		}
		if (asksRunAs) {
			deployDialogOpen = true
			return
		}
		await deployNow()
	}

	async function deployNow() {
		if (isNewApp) {
			const members = firstDeployMembers
			const workspace = opWorkspace!
			const path = newEditedPath
			// Read before the create: in a session pane `policy` is bound to the session's draft,
			// which the create's `onDeploy` discards.
			const links = { workspace, path, mode: accessModeOf(policy), customPath }
			if (await createApp(path)) {
				await applyFirstDeployMembers(workspace, 'raw_app', path, members)
				if (offersSharing) showAppLinks(links)
			}
		} else {
			await handleUpdateApp(newEditedPath)
		}
	}

	let onLatest = $state(true)
	/** The last comparison could not read the head, so the confirmation it raises is
	 *  caution and not an observed deploy. Cleared by the next reading comparison. */
	let headUnknown = $state(false)
	async function compareVersions() {
		if (version === undefined) {
			return
		}
		try {
			const appVersion = await AppService.getAppLatestVersion({
				workspace: opWorkspace!,
				path: appPath
			})
			onLatest = appVersion?.version === undefined || version === appVersion?.version
			headUnknown = false
		} catch (e) {
			console.error('Error comparing versions', e)
			// The head is what this compares against, so an unanswered read is not
			// evidence of being current: confirm, as an unclaimable deploy does, and say
			// that is why rather than claiming a version that was never seen.
			onLatest = false
			headUnknown = true
		}
	}

	const mod = isMac() ? '⌘' : 'Ctrl+'

	let moreItems = $derived([
		...(sidebarToggleInMenu && onToggleSidebar
			? [
					{
						displayName: `${sidebarCollapsed ? 'Expand' : 'Collapse'} file sidebar`,
						icon: sidebarCollapsed ? PanelLeft : PanelLeftClose,
						action: () => onToggleSidebar?.(),
						shortcut: `${mod}B`
					}
				]
			: []),
		...(phoneTopbar
			? [
					{
						displayName: 'Diff',
						icon: DiffIcon,
						action: () => openDiffDrawer(),
						disabled: !savedApp || newApp || savedApp?.no_deployed === true
					}
				]
			: []),
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
			displayName: 'Settings',
			icon: Settings,
			action: () => openSettings()
		},
		...(ownsPageHeader
			? [
					{
						displayName: 'Share',
						icon: UserPlus,
						disabled: neverDeployed,
						action: openShare
					}
				]
			: []),
		{
			displayName: 'Public URL',
			icon: Globe,
			disabled: neverDeployed,
			action: async () => {
				const secretUrl = await AppService.getPublicSecretOfApp({
					workspace: opWorkspace!,
					path: appPath
				})
				window.open(`${window.location.origin}/public/${opWorkspace}/${secretUrl}`, '_blank')
			}
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
			displayName: 'Record demo',
			icon: Circle,
			action: () => {
				recordDrawer?.openDrawer()
			},
			disabled: !savedApp
		}
	])

	let customPath = $state(savedApp?.custom_path)
	/** Share and the settings drawer write who can open the deployed app and its custom URL at
	 *  once: the editor follows, so its next deploy and diff see them. */
	function applyAppAccess(row: { policy: Policy; custom_path?: string | null }) {
		policy.execution_mode = row.policy.execution_mode
		customPath = row.custom_path ?? undefined
		if (savedApp) {
			savedApp.policy = { ...savedApp.policy, execution_mode: row.policy.execution_mode }
			savedApp.custom_path = row.custom_path ?? undefined
		}
	}
	// Derived, so a draft synced in from another session reaches the header; an edit here overrides
	// the value until the next one arrives.
	let labels = $derived(initialLabels)
	let settingsHaveErrors = $derived(pathError != '')

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
</script>

<DeployOnBehalfOfDialog
	kind="app"
	bind:open={deployDialogOpen}
	bind:message={() => deploymentMsg ?? '', (v) => (deploymentMsg = v)}
	bind:email={policy.on_behalf_of_email}
	bind:permissionedAs={policy.on_behalf_of}
	savedEmail={isNewApp ? undefined : deployedRunAs?.on_behalf_of_email}
	savedPermissionedAs={isNewApp ? undefined : deployedRunAs?.on_behalf_of}
	isNew={isNewApp}
	folderDefault={folderDefault.value}
	folderDefaultLoading={folderDefault.loading}
	executionMode={policy.execution_mode}
	targetWorkspace={opWorkspace ?? ''}
	rawApp
	onConfirmed={deployNow}
/>

<DeployOverrideConfirmationModal
	{deployedBy}
	{confirmCallback}
	bind:open
	{diffDrawer}
	claimOpening={() => (lastOpening = diffDrawer?.beginOpening())}
	{headUnknown}
	bind:deployedValue
	currentValue={currentDiffValue}
/>

<AppSettingsDrawer
	bind:this={settingsDrawer}
	bind:open={settingsOpen}
	{newPath}
	{newApp}
	bind:policy
	{appPath}
	{savedApp}
	rawApp
	operatingWorkspace={opWorkspace}
	bind:summary
	bind:pathError
	bind:newEditedPath
	bind:labels
	bind:customPath
	onAppChanged={applyAppAccess}
/>

{#if offersSharing}
	<ShareDialog bind:this={shareDialog} onAppChanged={applyAppAccess} />
	<FirstDeployDialog bind:this={firstDeployDialog} />
{/if}

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory on:restore={(e) => onRestore?.(e.detail)} {appPath} />
	</DrawerContent>
</Drawer>

<!-- Full screen: the demo is recorded at the size it replays at. -->
<Drawer bind:this={recordDrawer} size="100vw">
	<DrawerContent
		title="Record a demo — {savedApp?.path ?? appPath}"
		on:close={() => recordDrawer?.closeDrawer()}
	>
		<div class="flex flex-col h-full min-h-0 gap-2">
			<div class="text-xs text-secondary flex flex-col gap-1">
				<span>
					Use the app the way someone else would — each interaction becomes a step, and the
					recording captures the page as they would see it. Passwords are masked; add
					<span class="font-mono">data-wm-no-record</span> to anything else that should stay out.
				</span>
				<span>
					Stop recording, then <b>Download</b> the JSON. It is self-contained: open it on
					<a href="/replay" target="_blank" rel="noreferrer" class="text-blue-500 underline">
						/replay
					</a>
					— a public page that needs no login and can be embedded in an iframe. Host the JSON anywhere
					it can be fetched (S3, GitHub raw, your docs site) and link
					<span class="font-mono">/replay?src=&lt;url&gt;</span> to have it load itself. Windmill keeps
					no copy.
				</span>
			</div>
			<div class="flex-1 min-h-0">
				{#if recordDrawer}
					<!-- opWorkspace, not the selected one: a session edits an app that may
				     live in another workspace, and recording must load and run it there. -->
					<RawAppRecordSession workspace={opWorkspace ?? ''} path={savedApp?.path ?? appPath} />
				{/if}
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

{#if ownsPageHeader}
	<!-- The editor's own top bar is the page header on this route: the app's path and summary are
	     the breadcrumb's, and everything else the bar carried rides along as the header's actions.
	     `separator="none"`: the splitpanes below draw their own top line, and the bar's edge would
	     be a second one on top of it. -->
	<PageHeaderContent
		item={{
			// The path being edited, not the stored one: a brand-new app is parked at a
			// `draft_<uuid>` placeholder, and the trail would name that instead of the path Deploy
			// will create — which is why `newEditedPath` refuses the placeholder. Frozen while the
			// rename popover is open so the trail holds still as the user types.
			kind: 'app',
			path: pathSnapshot ?? (newEditedPath || appPath || newPath || undefined),
			summary,
			summaryContent: appSummary,
			pathTrigger: rawAppPathTrigger
		}}
		actions={rawAppHeaderActions}
		sessionSource={sessionOpen}
		actionFolds={4}
		contexts={headerContexts}
		separator="none"
	/>
{:else}
	<div
		bind:clientWidth={topbarWidth}
		class="flex flex-row justify-between gap-2 gap-y-2 px-2 items-center overflow-y-visible overflow-x-auto shrink-0 {condensedHeader
			? 'max-h-9 h-9'
			: 'max-h-12 h-12'}"
	>
		<!-- Identity block: shrinks/truncates first so the cloud indicator and the
	     action buttons stay visible. Without min-w-0 the breadcrumb + summary
	     overflow this box on narrow widths and get overlapped by the (formerly
	     un-pinned) action group, hiding the autosave cloud. -->
		<div class="flex flex-row gap-2 items-center min-w-0">
			{@render sidebarToggle()}
			<div class="min-w-0 overflow-hidden">
				<EditorHeader
					bind:summary
					bind:path={newEditedPath}
					savedPath={appPath || newPath || undefined}
					kind="app"
					raw_app
					hidePath={condensedHeader}
					workspaceId={autosaveWorkspace}
					onNavigate={(item) => (onNavigate ? onNavigate(item) : goto(editPathFor(item)))}
				/>
			</div>
			{@render autosaveIndicator()}
		</div>

		{@render awarenessMark()}
		<div class="flex flex-row gap-2 justify-end items-center overflow-visible shrink-0">
			{@render rawAppActions()}
		</div>
	</div>
{/if}

{#snippet sidebarToggle()}
	{#if onToggleSidebar && !sidebarToggleInMenu}
		<Button
			unifiedSize="sm"
			variant="subtle"
			iconOnly
			startIcon={{ icon: sidebarCollapsed ? PanelLeft : PanelLeftClose }}
			title={`${sidebarCollapsed ? 'Expand' : 'Collapse'} file sidebar (${isMac() ? '⌘' : 'Ctrl+'}B)`}
			on:click={() => onToggleSidebar?.()}
		/>
	{/if}
{/snippet}

{#snippet autosaveIndicator()}
	{#if opWorkspace && indicatorPath !== undefined}
		<AutosaveIndicator
			workspace={opWorkspace}
			itemKind="raw_app"
			path={indicatorPath}
			draftOnly={newApp}
			{onResetToDeployed}
			{loadedFromDraft}
			{othersDraftsCount}
			{onOpenOthersDrafts}
			compact={narrowTopbar}
		/>
	{/if}
{/snippet}

{#snippet awarenessMark()}
	{#if $enterpriseLicense && appPath != '' && !inSessionPane}
		<div class="shrink-0">
			<Awareness />
		</div>
	{/if}
{/snippet}

<!-- The summary's editor again, hung off the band's path segment so it opens under the path, with
     the cursor in the path field. `bind:` cannot be spread, so the slots are written out twice;
     both instances bind the same ones and only one is ever open. -->
{#snippet rawAppPathTrigger(pathLabel: Snippet, triggerClass: string)}
	<PathEditPopover
		label={pathLabel}
		{triggerClass}
		focusField="path"
		bind:summary
		bind:path={newEditedPath}
		bind:snapshotPath={pathSnapshot}
		savedPath={appPath || undefined}
		kind="app"
		workspaceId={autosaveWorkspace}
	/>
{/snippet}

{#snippet appSummary()}
	<!-- Not edited in place: clicking the name opens the summary and the path together, so the band
	     reads as a name rather than a form. `title` for one it truncates. -->
	<div class="flex items-center gap-1 min-w-0">
		{#snippet summaryText()}
			<span
				class="min-w-0 truncate text-xs {emptyString(summary)
					? 'text-tertiary italic font-normal'
					: 'font-medium text-emphasis'}"
				title={summary}>{emptyString(summary) ? 'Add a summary...' : summary}</span
			>
		{/snippet}
		<PathEditPopover
			label={summaryText}
			bind:summary
			bind:path={newEditedPath}
			bind:snapshotPath={pathSnapshot}
			savedPath={appPath || undefined}
			kind="app"
			workspaceId={autosaveWorkspace}
		/>
	</div>
{/snippet}

{#snippet rawAppHeaderActions()}
	<!-- The bar's own order kept: the file sidebar, how the draft is doing, who else is here, then
	     what to do with the app. The sidebar's toggle is the only way back once it is folded — the
	     collapsed sidebar renders nothing of its own — so it stays on the bar until the last fold,
	     which moves it into the ⋮ menu rather than dropping it. -->
	{@render sidebarToggle()}
	{@render autosaveIndicator()}
	{@render awarenessMark()}
	{@render rawAppActions()}
{/snippet}

{#snippet rawAppActions()}
	<DropdownV2 items={moreItems} class="h-auto">
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				unifiedSize={headerBtnSize}
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
		class={phoneTopbar
			? 'hidden'
			: !savedApp || newApp || savedApp?.no_deployed === true
				? 'flex cursor-not-allowed'
				: 'flex'}
	>
		<Button
			variant="default"
			unifiedSize={headerBtnSize}
			on:click={() => openDiffDrawer()}
			disabled={!savedApp || newApp || savedApp?.no_deployed === true}
			btnClasses={!savedApp || newApp || savedApp?.no_deployed === true
				? 'pointer-events-none'
				: undefined}
			iconOnly={compactTopbar && !ownsPageHeader}
			title={newApp || savedApp?.no_deployed === true
				? 'Deploy this app once to compare against the deployed version'
				: 'Diff'}
			startIcon={{ icon: DiffIcon }}
		>
			Diff
		</Button>
	</div>

	{#if !phoneTopbar}
		<div class="relative shrink-0">
			{#if settingsHaveErrors}
				<span class="absolute inline-flex rounded-full bg-red-600 h-2 w-2 z-50 -right-0.5 -top-0.5"
				></span>
			{/if}
			<Button
				variant="default"
				unifiedSize={headerBtnSize}
				startIcon={{ icon: Settings }}
				iconOnly={compactTopbar}
				title="Settings"
				on:click={() => openSettings()}
			>
				Settings
			</Button>
		</div>
	{/if}

	<div class="{compactTopbar ? 'hidden' : 'hidden md:inline'} relative overflow-visible">
		<Button
			on:click={() => {
				jobsDrawerOpen = true
			}}
			color="light"
			unifiedSize={headerBtnSize}
			variant="default"
			btnClasses="relative"
		>
			<div class="flex flex-row gap-1 items-center">
				<Bug size={14} />
				<div>Jobs</div>

				<div class="text-2xs text-primary">({jobs?.length > 99 ? '99+' : (jobs?.length ?? 0)})</div>
			</div>
		</Button>
	</div>
	<!-- Draws nothing in the bar: it is the export drawer, opened from the menu's Export entry
	     through this instance. -->
	<AppExportButton bind:this={appExport} />
	<!-- Right before Share and Deploy at every fold; only the opted-out docked-chat toggle gives way on a
	     phone. -->
	<OpenInSessionButton source={sessionOpen}>
		{#snippet fallback()}
			{#if !phoneTopbar}
				<Button
					unifiedSize={headerBtnSize}
					variant="default"
					onClick={() => aiChatManager.toggleOpen()}
					startIcon={{ icon: WandSparkles }}
					iconOnly
					btnClasses={AIBtnClasses('default')}
				>
					AI
				</Button>
			{/if}
		{/snippet}
	</OpenInSessionButton>
	{#if offersSharing && !phoneTopbar}
		<ShareButton
			deployed={!neverDeployed}
			iconOnly={compactTopbar}
			unifiedSize={headerBtnSize}
			onclick={openShare}
		/>
	{/if}
	<DeployButton
		bind:this={deployButton}
		loadingSave={loading.save}
		loading={app == undefined}
		on:save={({ detail }) => deploy(detail)}
		unifiedSize={headerBtnSize}
		dropdownItems={appPath != ''
			? () => [
					// `appPath`, not the live edited path: a rename that has not been deployed yet
					// would land on a details page for something that does not exist.
					...(newApp
						? []
						: [
								{
									label: 'Exit & see details',
									onClick: () => onDetails?.({ path: appPath })
								}
							]),
					{
						label: 'Fork',
						onClick: forkApp
					},
					...(!isCloudHosted() && editInForkAllowed(opWorkspace, $userWorkspaces)
						? [
								{
									label: editInForkLabel(opWorkspace, $userWorkspaces),
									onClick: () => {
										openEditInFork('raw_app', appPath, opWorkspace)
									}
								}
							]
						: [])
				]
			: undefined}
	/>
{/snippet}
