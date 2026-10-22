<script lang="ts">
	import { setDeployedOnBehalfOf } from '$lib/components/itemSettings/onBehalfOf'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import DeployOnBehalfOfDialog from '$lib/components/itemSettings/DeployOnBehalfOfDialog.svelte'
	import { useFolderDefaultPermissionedAs } from '$lib/components/useFolderDefaultPermissionedAs.svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Toggle from '$lib/components/Toggle.svelte'
	import { AppService, type Policy } from '$lib/gen'
	import { redo, undo } from '$lib/history.svelte'
	import { discardDraftAfterDeploy } from '$lib/userDraftToast'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { enterpriseLicense, userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { isMac, type Item, userPathPrefix, emptyString } from '$lib/utils'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import {
		AlignHorizontalSpaceAround,
		BellOff,
		Bug,
		DiffIcon,
		Expand,
		FileJson,
		FileUp,
		FormInput,
		History,
		Laptop2,
		Smartphone,
		FileClock,
		Sun,
		Moon,
		SunMoon,
		Undo,
		Redo,
		Zap,
		Globe,
		Settings,
		UserPlus
	} from 'lucide-svelte'
	import { getContext, onDestroy, untrack, type Snippet } from 'svelte'
	import { orderedJsonStringify, type Value, replaceFalseWithUndefined } from '../../../utils'
	import type { App, AppEditorContext, AppViewerContext } from '../types'
	import { toStatic } from '../utils'
	import AppExportButton from './AppExportButton.svelte'
	import AppInputs from './AppInputs.svelte'
	import PreviewToggle from './PreviewToggle.svelte'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from './DeploymentHistory.svelte'
	import Awareness from '$lib/components/Awareness.svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import AppReportsDrawer from './AppReportsDrawer.svelte'
	import DebugPanel from './contextPanel/DebugPanel.svelte'

	import EditorHeader from '$lib/components/EditorHeader.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import PathEditPopover from '$lib/components/PathEditPopover.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import AutosaveIndicator from '$lib/components/AutosaveIndicator.svelte'
	import { editPathFor } from '$lib/components/workspacePicker'
	import { invalidateWorkspacePaths } from '$lib/components/PathNameAutocomplete.svelte'
	import { beforeNavigate, goto } from '$app/navigation'
	import HideButton from './settingsPanel/HideButton.svelte'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import AppJobsDrawer from './AppJobsDrawer.svelte'
	import LazyModePanel from './contextPanel/LazyModePanel.svelte'
	import type { DiffDrawerI } from '$lib/components/diff_drawer'
	import AppSettingsDrawer, { type AppSettingsTab } from './AppSettingsDrawer.svelte'
	import DeployButton from '$lib/components/DeployButton.svelte'
	import { updatePolicy } from './appPolicy'
	import { appDraftSettings } from './appDraftSettings'
	import ShareDialog, { type ShareSection } from '$lib/components/itemSettings/ShareDialog.svelte'
	import FirstDeployDialog, {
		applyFirstDeployMembers
	} from '$lib/components/itemSettings/FirstDeployDialog.svelte'
	import ShareButton from '$lib/components/itemSettings/ShareButton.svelte'
	import { accessModeOf } from '$lib/components/itemSettings/appAccess'
	import type { ShareOpenSource } from '$lib/components/itemSettings/sharingTelemetry'
	import { classicAppDraftParts } from '$lib/appDiffSides'
	import { editInForkAllowed, editInForkLabel, openEditInFork } from '$lib/utils/editInFork'
	import { isCloudHosted } from '$lib/cloud'

	interface Props {
		policy: Policy
		fromHub?: boolean
		diffDrawer?: DiffDrawerI | undefined
		savedApp?:
			| {
					value: App
					path: string
					summary: string
					policy: any
					custom_path?: string
					labels?: string[]
			  }
			| undefined
		version?: number | undefined
		leftPanelHidden?: boolean
		rightPanelHidden?: boolean
		bottomPanelHidden?: boolean
		newApp: boolean
		newPath?: string
		/** Initial labels for the app, threaded from the loaded app data via AppEditor. */
		labels?: string[]
		/** URL path the draft is keyed under; empty on `/apps/add` (no draft yet). */
		userDraftPath?: string
		onSavedNewAppPath?: (path: string) => void
		onShowRightPanel?: () => void
		onShowLeftPanel?: () => void
		onShowBottomPanel?: () => void
		onHideRightPanel?: () => void
		onHideLeftPanel?: () => void
		onHideBottomPanel?: () => void
		onNavigate?: (item: import('$lib/components/workspacePicker').WorkspaceItem) => void
		// Threaded to the AutosaveIndicator's "Reset to deployed" button.
		onResetToDeployed?: () => void | Promise<void>
		// See ScriptBuilderProps — same indicator semantics.
		loadedFromDraft?: boolean
		othersDraftsCount?: number
		onOpenOthersDrafts?: () => void
		// Restoring an older deployment from the history drawer. A callback prop
		// (not `on:restore` forwarding): forwarding a `createEventDispatcher`
		// event up through these runes-mode components silently drops it.
		onRestore?: (restoredApp: any) => void
		// Fired after a successful deploy, which keeps this editor open: `version` is what
		// the deploy wrote, for the next draft's fork base, and `head` what is deployed
		// now, with its author and time.
		onDeploy?: (e: { version?: number; head?: number; headBy?: string; headAt?: string }) => void
		/** True for the route's own editor: its top bar becomes the page header. */
		ownsPageHeader?: boolean
	}

	let {
		policy = $bindable(),
		fromHub = false,
		diffDrawer = undefined,
		savedApp = $bindable(undefined),
		version = $bindable(undefined),
		leftPanelHidden = false,
		rightPanelHidden = false,
		bottomPanelHidden = false,
		newApp,
		newPath = '',
		labels: initialLabels = undefined,
		userDraftPath = '',
		onSavedNewAppPath,
		onShowLeftPanel,
		onShowRightPanel,
		onShowBottomPanel,
		onHideLeftPanel,
		onHideRightPanel,
		onHideBottomPanel,
		onNavigate = undefined,
		onResetToDeployed,
		loadedFromDraft = false,
		othersDraftsCount = 0,
		onOpenOthersDrafts,
		onRestore,
		onDeploy,
		ownsPageHeader = false
	}: Props = $props()

	/** Mirror of the path the user is editing in the rename popover. Initialized
	 * once from `newPath` (or a synthesized path for new apps) and only
	 * updated by user input from then on — we deliberately do NOT sync from
	 * `newPath` afterwards so the user's in-flight rename isn't clobbered by
	 * a parent reload that re-supplies the saved path. The fallback chain at
	 * read sites (`newEditedPath || savedApp?.path`)
	 * handles the case where `newEditedPath` is briefly empty before the
	 * synthesized initialization runs — falls through to the saved path so
	 * rename detection still works. */
	let newEditedPath = $state(
		untrack(() =>
			newApp && !newPath
				? userPathPrefix($userStore?.username) + random_adj() + '_app'
				: (newPath ?? '')
		)
	)
	let deployedValue: Value | undefined = $state(undefined) // Value to diff against
	let deployedBy: string | undefined = $state(undefined) // Author
	let confirmCallback: () => void = $state(() => {}) // What happens when user clicks `override` in warning
	let open: boolean = $state(false) // Is confirmation modal open

	const {
		app,
		summary,
		breakpoint,
		appPath,
		jobs,
		jobsById,
		staticExporter,
		errorByComponent,
		openDebugRun,
		mode,
		darkMode
	} = getContext<AppViewerContext>('AppViewerContext')

	// Mirror the user-typed path into the draft as `draft_path` when it differs
	// from the baseline, so the home row shows the friendly name instead of
	// `draft_{uuid}`. Drop the field once it matches the baseline again.
	$effect(() => {
		const typed = newEditedPath
		const baseline = savedApp?.path ?? ''
		const a = $app
		if (!a) return
		untrack(() => {
			if (typed && typed !== baseline) {
				a.draft_path = typed
			} else if (a.draft_path !== undefined) {
				delete a.draft_path
			}
		})
	})

	// Mirror the summary and the app settings onto the autosaved App so a draft
	// persists them (the autosave stores the bare App value, which has none of
	// its own — they live in `app` table columns, set only on deploy). Without
	// this they are lost when reopening a draft or deploying it from the Review &
	// Deploy page. Parallels `draft_path`.
	$effect(() => {
		const s = $summary
		const settings = appDraftSettings(
			{
				policy: $state.snapshot(policy),
				labels: $state.snapshot(labels),
				custom_path: customPath
			},
			!newApp
		)
		const a = $app
		if (!a) return
		untrack(() => {
			if (a.summary !== s) a.summary = s
			// Compared first: every write reaches the autosave.
			const same = (x: unknown, y: unknown) => orderedJsonStringify(x) === orderedJsonStringify(y)
			if (!same(a.policy, settings.policy)) a.policy = settings.policy
			if (!same(a.labels, settings.labels)) a.labels = settings.labels
			if (a.custom_path !== settings.custom_path) a.custom_path = settings.custom_path
		})
	})

	const { history, jobsDrawerOpen, refreshComponents } =
		getContext<AppEditorContext>('AppEditorContext')

	// Sessions inject an AIChatManager via context; AppEditor skips its
	// UserDraft handle in that case, so the cleanup calls here must skip too
	// (otherwise we'd wipe a non-session tab's autosave at the same path). The
	// session-side equivalent is the View's `onDeploy` →
	// `runtime.itemDeployed`, which discards the fork draft + reloads
	// the preview to the deployed version.
	const inSessionPane = !!getContext('aiChatManager')

	// The header's buttons render under the page header, not under this component, so the contexts
	// they look up have to travel with them. The app's own two carry everything the canvas toggles,
	// the panel buttons, Debug runs and the preview switch read.
	/** Held by the rename popover while it is open; the band's trail reads it so the path does not
	 *  reflow under the pointer as the user types. */
	let pathSnapshot = $state<string | undefined>(undefined)

	const headerContexts = new Map<any, any>([
		['AppViewerContext', getContext('AppViewerContext')],
		['AppEditorContext', getContext('AppEditorContext')]
	])

	const loading = $state({
		publish: false,
		save: false
	})

	let selectedJobId: string | undefined = $state(undefined)

	let pathError: string = $state('')

	let appExport: AppExportButton | undefined = $state()

	let settingsOpen = $state(false)
	let settingsTab: AppSettingsTab = $state('metadata')
	let inputsDrawerOpen = $state(untrack(() => fromHub))
	let historyBrowserDrawerOpen = $state(false)
	let debugAppDrawerOpen = $state(false)
	let lazyDrawerOpen = $state(false)
	let deploymentMsg = $state('')
	const operatingUser = useOperatingUser()
	// Only the app's first deploy takes the folder's default user.
	const folderDefault = useFolderDefaultPermissionedAs(
		() => (newApp ? newEditedPath : undefined),
		() => $workspaceStore
	)
	let deployDialogOpen = $state(false)

	// Top-bar responsive collapse — container width, not viewport.
	let topbarWidth = $state(0)
	// In the page header the bar folds the buttons (`actionFolds` below), once its breadcrumb has
	// given all it can; anywhere else this editor's own top bar is theirs alone. Folded, the canvas
	// toggles stand down and the menu carries them and Debug runs, which keeps them reachable
	// rather than merely out of the way.
	const compactTopbar = $derived(
		ownsPageHeader ? pageHeader.actionFold >= 1 : topbarWidth > 0 && topbarWidth < 720
	)
	// The second fold holds the trail and Deploy. Which panels are open, the export and the
	// editor/preview switch join the menu compact already shows.
	const phoneTopbar = $derived(ownsPageHeader && pageHeader.actionFold >= 2)

	// The draft-only fields mirrored onto `$app` (fork base, staged path, summary and settings)
	// are columns of the app row or editor state. Deployed inside the value, they would land in
	// git-synced files and version diffs, `parent_version` changing on every deploy; exported,
	// they would hand out the publisher's email and the custom URL.
	function withoutDraftBase<T extends App | undefined>(a: T): T {
		if (!a) return a
		return classicAppDraftParts(a).value as T
	}

	/** Resolves to whether the app was created. */
	async function createApp(path: string): Promise<boolean> {
		// In place: the settings drawer and the draft mirror hold this object.
		Object.assign(policy, await updatePolicy($app, policy))
		// Read for the path being created: the first-deploy dialog may have just chosen it.
		setDeployedOnBehalfOf(policy, operatingUser.current, await folderDefault.refetch())
		try {
			await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: withoutDraftBase($app),
					path,
					summary: $summary,
					policy,
					deployment_message: deploymentMsg,
					custom_path: customPath,
					// The user the deploy dialog confirmed, which the backend keeps only for a deployer
					// who may preserve one.
					preserve_on_behalf_of: true,
					labels
				}
			})
			// New path now exists server-side — drop the autocomplete cache so
			// it shows up immediately instead of after the 60s TTL.
			invalidateWorkspacePaths($workspaceStore!)
			savedApp = {
				summary: $summary,
				value: structuredClone($state.snapshot($app)),
				path: path,
				policy: $state.snapshot(policy),
				custom_path: customPath,
				labels: $state.snapshot(labels)
			}
			sendUserToast('App deployed successfully')
			// Remove the autosave at its canonical URL draft key, not the deploy
			// `path` (they differ for new apps), or the real draft row orphans.
			// stopSync-bracketed + flushed so AppEditor's mirror can't re-save.
			if (!inSessionPane && $workspaceStore) {
				discardDraftAfterDeploy({
					workspace: $workspaceStore,
					itemKind: 'app',
					path: userDraftPath
				})
			}
			onSavedNewAppPath?.(path)
			return true
		} catch (e) {
			sendUserToast('Error creating app', e)
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
				$app &&
				orderedJsonStringify(deployedValue) ===
					orderedJsonStringify(
						replaceFalseWithUndefined({
							summary: $summary,
							value: withoutDraftBase($app),
							path: newEditedPath || savedApp.path,
							policy,
							custom_path: customPath,
							labels
						})
					)
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

	/** The opening this editor claimed last. A path change remounts this editor while the
	 *  drawer stays mounted, so its teardown hands that opening back rather than leaving
	 *  the drawer on the app the user left. */
	let lastOpening: number | undefined = undefined
	onDestroy(() => {
		if (lastOpening != null) diffDrawer?.abandonOpening(lastOpening)
	})

	async function syncWithDeployed(opening?: number) {
		const deployedApp = await AppService.getAppByPath({
			workspace: $workspaceStore!,
			path: $appPath!,
			withStarredInfo: true
		})

		// A superseded opening must not write these: the current one would then render
		// against the older deployed value.
		if (opening != null && !diffDrawer?.ownsOpening(opening)) return
		deployedBy = deployedApp.created_by

		// Strip off extra information
		deployedValue = replaceFalseWithUndefined({
			...deployedApp,
			value: withoutDraftBase(deployedApp.value as App),
			id: undefined,
			created_at: undefined,
			created_by: undefined,
			versions: undefined,
			extra_perms: undefined
		})
	}

	async function updateApp(npath: string) {
		// Who can open the app is set from the Share dialog and applies at once. Read here rather
		// than at load, so a change made since then is not undone; an omitted mode would reset an
		// operator-built app to the publisher's.
		const deployedRow = await AppService.getAppLiteByPath({
			workspace: $workspaceStore!,
			path: $appPath!
		})
		policy.execution_mode = deployedRow.policy.execution_mode
		// In place: the settings drawer and the draft mirror hold this object.
		Object.assign(policy, await updatePolicy($app, policy))
		setDeployedOnBehalfOf(policy, operatingUser.current)
		const deployed = await AppService.updateApp({
			workspace: $workspaceStore!,
			path: $appPath!,
			requestBody: {
				value: withoutDraftBase($app!),
				summary: $summary,
				policy,
				path: npath,
				deployment_message: deploymentMsg,
				// Absent keeps the custom URL, which the Share dialog sets.
				custom_path: undefined,
				preserve_on_behalf_of: true,
				labels
			}
		})
		invalidateWorkspacePaths($workspaceStore!)
		savedApp = {
			summary: $summary,
			value: structuredClone($state.snapshot($app)),
			path: npath,
			policy: $state.snapshot(policy),
			custom_path: customPath,
			labels: $state.snapshot(labels)
		}
		const head = await AppService.getAppLatestVersion({
			workspace: $workspaceStore!,
			path: npath
		})
		// `version` is what is deployed now, which is the deploy guard's fallback head; the
		// deploy's own answer is the base, and the two differ when another landed beside it.
		version = head?.version
		// Re-pin the fork base to the version just written: the editor stays open, so a
		// follow-up deploy (or a new edit) would otherwise compare against the now-
		// superseded base and falsely warn. parent_version is in
		// DRAFT_COMPARE_IGNORED_FIELDS, so this write can't spawn a spurious draft.
		if ($app) $app.parent_version = deployed.version
		// The route owns the pair the out-of-date prompt reads, and this editor stays open
		// across the deploy, so hand both over rather than leaving it on the old ones.
		onDeploy?.({
			version: deployed.version,
			head: version,
			headBy: head?.created_by,
			headAt: head?.created_at
		})

		sendUserToast('App deployed successfully')
		// Canonical autosave key (the URL draft path), not `$appPath` — a
		// rename leaves the autosave at the original key, so removing at
		// `$appPath` would miss it. Bracketed + flushed (see createApp).
		if (!inSessionPane && $workspaceStore) {
			discardDraftAfterDeploy({
				workspace: $workspaceStore,
				itemKind: 'app',
				path: userDraftPath
			})
		}
		if ($appPath !== npath) {
			onSavedNewAppPath?.(npath)
		}
	}

	function openSettings(tab: AppSettingsTab) {
		settingsTab = tab
		settingsOpen = true
	}

	let shareDialog: ShareDialog | undefined = $state()
	let firstDeployDialog: FirstDeployDialog | undefined = $state()

	function openShare(source: ShareOpenSource, section?: ShareSection) {
		shareDialog?.openDialog('app', $appPath!, { source, section, fromEditor: true })
	}

	// Chosen in the first-deploy dialog, added once the run-as confirmation has created the app.
	let firstDeployMembers: Record<string, boolean> = {}

	async function deploy(message?: string) {
		if (pathError != '') {
			openSettings('metadata')
			return
		}
		deploymentMsg = message ?? ''
		firstDeployMembers = {}
		if (newApp && ownsPageHeader) {
			// The dialog also asks who the app runs on behalf of, for the path it chose.
			const choice = await firstDeployDialog?.ask({
				kind: 'app',
				summary: $summary,
				path: newEditedPath,
				appMode: accessModeOf(policy),
				runAs: {
					email: policy.on_behalf_of_email,
					permissionedAs: policy.on_behalf_of,
					targetWorkspace: $workspaceStore ?? ''
				}
			})
			if (!choice) return
			$summary = choice.summary
			newEditedPath = choice.path
			policy.execution_mode = choice.appMode
			if (choice.runAs) {
				policy.on_behalf_of_email = choice.runAs.email
				policy.on_behalf_of = choice.runAs.permissionedAs
			}
			deploymentMsg = choice.message
			firstDeployMembers = choice.members
			await deployNow()
			return
		}
		deployDialogOpen = true
	}

	async function deployNow() {
		// Use `newApp` (set for /apps/add and draft-only paths), not
		// `$appPath` — its `draft_{uuid}` is a poor create signal.
		if (newApp) {
			const members = firstDeployMembers
			if (await createApp(newEditedPath)) {
				await applyFirstDeployMembers($workspaceStore!, 'app', newEditedPath, members)
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
		// Compare the draft's pinned fork base (`$app.parent_version`) against the
		// current head when editing a draft, else the load-time head. Catches both a
		// concurrent deploy (head moved since open) AND a stale draft reopened after a
		// deploy (head == load-time head, but the draft was forked from an older one).
		const base = $app?.parent_version ?? version
		if (base === undefined) {
			return
		}
		try {
			const appVersion = await AppService.getAppLatestVersion({
				workspace: $workspaceStore!,
				path: $appPath
			})
			onLatest = appVersion?.version === undefined || base === appVersion?.version
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

	/** Flush the pending autosave (also covers the toggle-off parked case).
	 * Returns whether there was a draft to flush — false in the AI session pane,
	 * which owns no handle. */
	function flushDraft(): boolean {
		if (inSessionPane || !$workspaceStore || !userDraftPath) return false
		void UserDraftDbSyncer.flush({
			workspace: $workspaceStore,
			itemKind: 'app',
			path: userDraftPath
		})
		return true
	}

	// Monaco swallows the keydown, so an inline script or template editor with
	// focus never reaches the window handler below; Editor/SimpleEditor/
	// TemplateEditor re-broadcast it (untyped event, hence the manual listener).
	$effect(() => {
		const onMonacoSave = () => void flushDraft()
		window.addEventListener('wm-monaco-save-shortcut', onMonacoSave)
		return () => window.removeEventListener('wm-monaco-save-shortcut', onMonacoSave)
	})

	let lock = false
	function onKeyDown(event: KeyboardEvent) {
		if (lock) return

		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}

		lock = true

		// Only lowercase single-char keys — named keys (`ArrowDown`, etc.) must
		// stay PascalCase to match their switch cases.
		switch (event.key.length === 1 ? event.key.toLowerCase() : event.key) {
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					const napp = event.shiftKey ? redo(history) : undo(history, $app)
					for (const key in napp) {
						$app[key] = napp[key]
					}
					event.preventDefault()
				}
				break
			case 's':
				// Shift excluded: the switch lowercases so Ctrl+Shift+S lands here
				// too, and swallowing it would steal the browser/OS shortcut.
				// Swallowed only when there is a draft to flush, so the contexts
				// that can't act on it (AI session pane) leave the key alone.
				if ((event.ctrlKey || event.metaKey) && !event.shiftKey && flushDraft()) {
					event.preventDefault()
				}
				break
			// case 'ArrowDown': {
			// 	let ids = generateIds()
			// 	let idx = ids.indexOf($selectedIdStore)
			// 	if (idx > -1 && idx < ids.length - 1) {
			// 		$selectedIdStore = ids[idx + 1]
			// 		event.preventDefault()
			// 	}
			// 	break
			// }
			// case 'ArrowUp': {
			// 	let ids = generateIds()
			// 	let idx = ids.indexOf($selectedIdStore)
			// 	if (idx > 0 && idx < ids.length) {
			// 		$selectedIdStore = ids[idx - 1]
			// 		event.preventDefault()
			// 	}
			// 	break
			// }
		}
		lock = false
	}

	const mod = isMac() ? '⌘' : 'Ctrl+'

	function handleUndo() {
		const napp = undo(history, $app)
		for (const key in napp) {
			$app[key] = napp[key]
		}
	}
	function handleRedo() {
		const napp = redo(history)
		for (const key in napp) {
			$app[key] = napp[key]
		}
	}

	let moreItems = $derived([
		...(compactTopbar
			? [
					{
						displayName: `Debug runs (${$jobs?.length > 99 ? '99+' : ($jobs?.length ?? 0)})`,
						icon: Bug,
						action: () => {
							if (selectedJobId == undefined && $jobs.length > 0) {
								selectedJobId = $jobs[$jobs.length - 1]
							}
							$jobsDrawerOpen = true
						}
					},
					// The canvas toggles stand down with the bar, and the breakpoint among them is the
					// only way onto the mobile layout — so each one keeps an entry here.
					{
						displayName: $breakpoint === 'sm' ? 'Computer view' : 'Mobile view',
						icon: $breakpoint === 'sm' ? Laptop2 : Smartphone,
						action: () => ($breakpoint = $breakpoint === 'sm' ? 'lg' : 'sm')
					},
					// The setting the mobile view is edited for, which otherwise lives beside the
					// toggle group that stood down: without it here a deployed app could be put on
					// the mobile layout at this width and never taken off it.
					...($breakpoint === 'sm'
						? [
								{
									displayName: $app?.mobileViewOnSmallerScreens
										? 'Mobile view on small screens: on'
										: 'Mobile view on small screens: off',
									icon: Smartphone,
									action: () => {
										if ($app) $app.mobileViewOnSmallerScreens = !$app.mobileViewOnSmallerScreens
									}
								}
							]
						: []),
					{
						displayName: $app?.fullscreen ? 'Centered canvas' : 'Full-width canvas',
						icon: $app?.fullscreen ? AlignHorizontalSpaceAround : Expand,
						action: () => {
							if ($app) $app.fullscreen = !$app.fullscreen
						}
					},
					{
						displayName:
							$app?.darkMode === undefined
								? 'Theme: automatic'
								: $app.darkMode
									? 'Theme: dark'
									: 'Theme: light',
						icon: $app?.darkMode === undefined ? SunMoon : $app.darkMode ? Moon : Sun,
						// Cycles auto → light → dark, the order the toggle group reads in.
						action: () =>
							setTheme($app?.darkMode === undefined ? false : $app.darkMode ? undefined : true),
						separatorBottom: true
					}
				]
			: []),
		{
			displayName: 'Undo',
			icon: Undo,
			action: () => handleUndo(),
			disabled: $history?.index === 0,
			shortcut: `${mod}Z`
		},
		{
			displayName: 'Redo',
			icon: Redo,
			action: () => handleRedo(),
			disabled: $history && $history?.index === $history.history.length - 1,
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
				appExport?.open(withoutDraftBase($app))
			}
		},
		{
			displayName: 'Settings',
			icon: Settings,
			action: () => openSettings('metadata')
		},
		...(ownsPageHeader
			? [
					{
						displayName: 'Share',
						icon: UserPlus,
						disabled: newApp,
						action: () => openShare('editor_header')
					},
					{
						displayName: 'Public URL',
						icon: Globe,
						disabled: newApp,
						action: () => openShare('public_url', 'app')
					}
				]
			: []),
		{
			displayName: 'App inputs',
			icon: FormInput,
			action: () => {
				inputsDrawerOpen = true
			}
		},
		{
			displayName: 'Schedule reports',
			icon: FileClock,
			action: () => {
				appReportingDrawerOpen = true
			},
			disabled: !savedApp
		},
		{
			displayName: 'Diff',
			icon: DiffIcon,
			action: async () => {
				if (!savedApp || newApp) {
					return
				}
				// The fetch below is awaited, so a reopen (or a path change, which remounts
				// this editor but not the drawer) while it runs must not have the older one
				// land last. The drawer counts the openings for that reason.
				const opening = diffDrawer?.beginOpening()
				lastOpening = opening
				if (opening == null) return

				// deployedValue should be syncronized when we open Diff
				await syncWithDeployed(opening)

				if (!diffDrawer?.ownsOpening(opening)) return
				diffDrawer.openDrawer(opening)
				diffDrawer.setDiff(
					{
						mode: 'normal',
						deployed: deployedValue ?? savedApp,
						current: {
							summary: $summary,
							value: withoutDraftBase($app),
							path: newEditedPath || savedApp.path,
							policy,
							custom_path: customPath,
							labels
						},
						button: {
							text: 'Looks good, deploy',
							onClick: () => deploy(deploymentMsg)
						}
					},
					opening
				)
			},
			disabled: !savedApp || newApp
		},
		// App debug menu
		{
			displayName: 'Troubleshoot panel',
			icon: Bug,
			action: () => {
				debugAppDrawerOpen = true
			}
		},
		{
			displayName: 'Lazy mode',
			icon: Zap,
			action: () => {
				lazyDrawerOpen = true
			}
		},
		{
			displayName: 'Hub export',
			icon: FileUp,
			action: () => {
				appExport?.open(toStatic(withoutDraftBase($app), $staticExporter, $summary).app)
			}
		}
	]) as Item[]

	let appReportingDrawerOpen = $state(false)

	export function openTroubleshootPanel() {
		debugAppDrawerOpen = true
	}

	function setTheme(newDarkMode: boolean | undefined) {
		let globalDarkMode = window.localStorage.getItem('dark-mode')
			? window.localStorage.getItem('dark-mode') === 'dark'
			: window.matchMedia('(prefers-color-scheme: dark)').matches
		if (newDarkMode === true || (newDarkMode === null && globalDarkMode)) {
			document.documentElement.classList.add('dark')
		} else if (newDarkMode === false) {
			document.documentElement.classList.remove('dark')
		}
		$darkMode = newDarkMode ?? globalDarkMode
		$app.darkMode = newDarkMode
	}

	let priorDarkMode = document.documentElement.classList.contains('dark')
	setTheme($app?.darkMode)

	// Restore the user's prior theme on navigation away from the editor; the
	// app's darkMode override would otherwise leak into the next page.
	beforeNavigate(() => {
		setTheme(priorDarkMode)
	})

	// A deployed app's custom URL is set from the Share dialog, never drafted; a never-deployed
	// app's draft carries the one its first deploy will create it with.
	let customPath = $state(
		untrack(() => (newApp ? ($app?.custom_path ?? undefined) : savedApp?.custom_path))
	)
	let labels = $state(untrack(() => initialLabels))

	$effect(() => {
		if ($openDebugRun == undefined) {
			$openDebugRun = (jobId: string) => {
				$jobsDrawerOpen = true
				selectedJobId = jobId
			}
		}
	})

	let settingsHaveErrors = $derived(pathError != '')
	let hasErrors = $derived(Object.keys($errorByComponent).length > 0)
</script>

<svelte:window onkeydown={onKeyDown} />

<DeployOnBehalfOfDialog
	kind="app"
	bind:open={deployDialogOpen}
	bind:message={deploymentMsg}
	bind:email={policy.on_behalf_of_email}
	bind:permissionedAs={policy.on_behalf_of}
	savedEmail={newApp ? undefined : savedApp?.policy?.on_behalf_of_email}
	savedPermissionedAs={newApp ? undefined : savedApp?.policy?.on_behalf_of}
	isNew={newApp}
	folderDefault={folderDefault.value}
	executionMode={policy.execution_mode}
	targetWorkspace={$workspaceStore ?? ''}
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
	currentValue={{
		summary: $summary,
		value: withoutDraftBase($app),
		path: newEditedPath || savedApp?.path,
		policy,
		custom_path: customPath,
		labels
	}}
/>

<AppJobsDrawer
	bind:open={$jobsDrawerOpen}
	jobs={$jobs}
	on:clear={() => {
		$jobs = []
		$errorByComponent = {}
	}}
	on:clearErrors={() => {
		$errorByComponent = {}
	}}
	{hasErrors}
	{selectedJobId}
	refreshComponents={$refreshComponents}
	jobsById={$jobsById}
	errorByComponent={$errorByComponent}
/>
<AppSettingsDrawer
	bind:open={settingsOpen}
	bind:tab={settingsTab}
	{newPath}
	{newApp}
	{policy}
	appPath={$appPath}
	{savedApp}
	bind:summary={$summary}
	bind:pathError
	bind:newEditedPath
	bind:labels
/>

{#if ownsPageHeader}
	<ShareDialog
		bind:this={shareDialog}
		onAppChanged={(row) => {
			policy.execution_mode = row.policy.execution_mode
			customPath = row.custom_path ?? undefined
			if (savedApp) {
				savedApp.policy = { ...savedApp.policy, execution_mode: row.policy.execution_mode }
				savedApp.custom_path = row.custom_path ?? undefined
			}
		}}
	/>
	<FirstDeployDialog bind:this={firstDeployDialog} />
{/if}

<Drawer bind:open={inputsDrawerOpen} size="600px">
	<DrawerContent title="App inputs configuration" on:close={() => (inputsDrawerOpen = false)}>
		<AppInputs />
	</DrawerContent>
</Drawer>

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory on:restore={(e) => onRestore?.(e.detail)} appPath={$appPath} />
	</DrawerContent>
</Drawer>

<Drawer bind:open={debugAppDrawerOpen} size="800px">
	<DrawerContent title="Troubleshoot Panel" on:close={() => (debugAppDrawerOpen = false)}>
		<DebugPanel />
	</DrawerContent>
</Drawer>

<Drawer bind:open={lazyDrawerOpen} size="800px">
	<DrawerContent title="Lazy Mode" on:close={() => (lazyDrawerOpen = false)}>
		<LazyModePanel />
	</DrawerContent>
</Drawer>

<AppReportsDrawer bind:open={appReportingDrawerOpen} appPath={$appPath ?? ''} />

{#if ownsPageHeader}
	<!-- The editor's own top bar is the page header on this route: the app's path and summary are
	     the breadcrumb's, and everything else the bar carried rides along as the header's actions. -->
	<PageHeaderContent
		item={{
			// The path being edited, not the stored one: a brand-new app is parked at a
			// `draft_<uuid>` placeholder, and the trail would name that instead of the path Deploy
			// will create. Same fallback chain the rename sites read.
			// Frozen while the rename popover is open so the trail holds still as the user types.
			kind: 'app',
			path: pathSnapshot ?? (newEditedPath || $appPath || newPath || undefined),
			summary: $summary,
			summaryContent: appSummary,
			pathTrigger: appPathTrigger
		}}
		actions={appHeaderActions}
		actionFolds={2}
		contexts={headerContexts}
		separator="always"
	/>
{:else}
	<div
		bind:clientWidth={topbarWidth}
		class="flex flex-row justify-between gap-2 gap-y-2 px-2 items-center overflow-y-visible overflow-x-auto max-h-12 h-12 shrink-0"
	>
		<!-- Identity block shrinks/truncates first (min-w-0) so the cloud indicator
	     and the pinned action groups (shrink-0 below) stay visible on narrow
	     widths instead of the breadcrumb overflowing and hiding them. Kept at
	     min-w-0 (not flex-1) so justify-between still positions the panel-hide
	     buttons between the identity and the actions. -->
		<div class="flex flex-row gap-2 items-center min-w-0">
			<div class="min-w-0 overflow-hidden">
				<EditorHeader
					bind:summary={$summary}
					bind:path={newEditedPath}
					savedPath={$appPath || newPath || undefined}
					kind="app"
					onNavigate={(item) => (onNavigate ? onNavigate(item) : goto(editPathFor(item)))}
				/>
			</div>
			{@render canvasToggles()}
			{@render autosaveIndicator()}
		</div>

		{@render panelButtons()}
		{@render awarenessMark()}
		{@render appActions()}
	</div>
{/if}

<!-- The summary's editor again, hung off the band's path segment so it opens under the path, with
     the cursor in the path field. `bind:` cannot be spread, so the slots are written out twice;
     both instances bind the same ones and only one is ever open. -->
{#snippet appPathTrigger(pathLabel: Snippet, triggerClass: string)}
	<PathEditPopover
		label={pathLabel}
		{triggerClass}
		focusField="path"
		bind:summary={$summary}
		bind:path={newEditedPath}
		bind:snapshotPath={pathSnapshot}
		savedPath={$appPath || newPath || undefined}
		kind="app"
	/>
{/snippet}

{#snippet appSummary()}
	<!-- Not edited in place: clicking the name opens the summary and the path together, so the band
	     reads as a name rather than a form. `title` for one it truncates. -->
	<div class="flex items-center gap-1 min-w-0">
		{#snippet summaryText()}
			<span
				class="min-w-0 truncate text-xs {emptyString($summary)
					? 'text-tertiary italic font-normal'
					: 'font-medium text-emphasis'}"
				title={$summary}>{emptyString($summary) ? 'Add a summary...' : $summary}</span
			>
		{/snippet}
		<PathEditPopover
			label={summaryText}
			bind:summary={$summary}
			bind:path={newEditedPath}
			bind:snapshotPath={pathSnapshot}
			savedPath={$appPath || newPath || undefined}
			kind="app"
		/>
	</div>
{/snippet}

{#snippet appHeaderActions()}
	<!-- The bar's own order kept: what the canvas looks like, how the draft is doing, which panels
	     are open, then what to do with the app. The set scrolls rather than pushing past the end of
	     the bar — this editor carries the widest action row of any page, and the row it replaced
	     scrolled for the same reason. -->
	<div
		class="flex flex-row items-center gap-2 min-w-0 overflow-x-auto scrollbar-hidden whitespace-nowrap"
	>
		{@render canvasToggles()}
		{@render autosaveIndicator()}
		{#if !phoneTopbar}
			{@render panelButtons()}
			{@render awarenessMark()}
		{/if}
		{@render appActions()}
	</div>
{/snippet}

{#snippet canvasToggles()}
	<div class="flex gap-2 shrink-0 {compactTopbar ? 'hidden' : ''}">
		{#if $app}
			<ToggleButtonGroup
				selected={$app.fullscreen ? 'true' : 'false'}
				on:selected={({ detail }) => {
					$app.fullscreen = detail === 'true'
				}}
			>
				{#snippet children({ item })}
					<ToggleButton
						icon={AlignHorizontalSpaceAround}
						iconOnly
						value={'false'}
						tooltip="The max width is 1168px and the content stay centered instead of taking the full page width"
						{item}
						size="md"
					/>
					<ToggleButton
						tooltip="The width is of the app if the full width of its container"
						icon={Expand}
						iconOnly
						value={'true'}
						{item}
						size="md"
					/>
				{/snippet}
			</ToggleButtonGroup>
		{/if}
		{#if $app}
			<ToggleButtonGroup
				on:selected={({ detail }) => {
					const theme = detail === 'dark' ? true : detail === 'sun' ? false : undefined
					setTheme(theme)
				}}
				selected={$app.darkMode === undefined ? 'auto' : $app.darkMode ? 'dark' : 'sun'}
			>
				{#snippet children({ item })}
					<ToggleButton
						icon={SunMoon}
						iconOnly
						value={'auto'}
						tooltip="The app mode between dark/light is automatic"
						{item}
						size="md"
					/>
					<ToggleButton
						icon={Sun}
						iconOnly
						value={'sun'}
						tooltip="Force light mode"
						{item}
						size="md"
					/>
					<ToggleButton
						tooltip="Force dark mode"
						icon={Moon}
						iconOnly
						value={'dark'}
						{item}
						size="md"
					/>
				{/snippet}
			</ToggleButtonGroup>
		{/if}
		<div class="flex flex-row gap-2">
			<ToggleButtonGroup bind:selected={$breakpoint}>
				{#snippet children({ item })}
					<ToggleButton
						tooltip="Computer View"
						icon={Laptop2}
						iconOnly
						value={'lg'}
						{item}
						size="md"
					/>
					<ToggleButton
						tooltip="Mobile View"
						icon={Smartphone}
						iconOnly
						value={'sm'}
						{item}
						size="md"
					/>
					{#if $breakpoint === 'sm'}
						<Toggle
							size="xs"
							options={{
								right: 'Enable mobile view for smaller screens',
								rightTooltip:
									'Desktop view is enabled by default. Enable this to customize the layout of the components for the mobile view'
							}}
							textClass="text-2xs whitespace-nowrap white !w-full"
							bind:checked={$app.mobileViewOnSmallerScreens}
							class="flex flex-row px-2 items-center"
						/>
					{/if}
				{/snippet}
			</ToggleButtonGroup>
		</div>
	</div>
{/snippet}

{#snippet autosaveIndicator()}
	{#if $workspaceStore}
		<div class="ml-4">
			<AutosaveIndicator
				workspace={$workspaceStore}
				itemKind="app"
				path={userDraftPath}
				draftOnly={newApp}
				{onResetToDeployed}
				{loadedFromDraft}
				{othersDraftsCount}
				{onOpenOthersDrafts}
			/>
		</div>
	{/if}
{/snippet}

{#snippet panelButtons()}
	{#if $mode !== 'preview'}
		<div class="flex gap-1 shrink-0">
			<HideButton
				direction="left"
				hidden={leftPanelHidden}
				on:click={() => {
					if (leftPanelHidden) {
						onShowLeftPanel?.()
					} else {
						onHideLeftPanel?.()
					}
				}}
			/>
			<HideButton
				hidden={bottomPanelHidden}
				direction="bottom"
				on:click={() => {
					if (bottomPanelHidden) {
						onShowBottomPanel?.()
					} else {
						onHideBottomPanel?.()
					}
				}}
			/>
			<HideButton
				hidden={rightPanelHidden}
				direction="right"
				on:click={() => {
					if (rightPanelHidden) {
						onShowRightPanel?.()
					} else {
						onHideRightPanel?.()
					}
				}}
			/>
		</div>
	{/if}
{/snippet}

{#snippet awarenessMark()}
	{#if $enterpriseLicense && $appPath != '' && !inSessionPane}
		<div class="shrink-0">
			<Awareness />
		</div>
	{/if}
{/snippet}

{#snippet appActions()}
	<div class="flex flex-row gap-2 justify-end items-center overflow-visible shrink-0">
		<Dropdown items={moreItems} />

		<div class="{compactTopbar ? 'hidden' : 'hidden md:inline'} relative overflow-visible shrink-0">
			{#if hasErrors}
				<span
					class="animate-ping absolute inline-flex rounded-full bg-red-600 h-2 w-2 z-50 -right-0.5 -top-0.5"
				></span>
				<span class=" absolute inline-flex rounded-full bg-red-600 h-2 w-2 z-50 -right-0.5 -top-0.5"
				></span>
			{/if}
			<Button
				on:click={() => {
					if (selectedJobId == undefined && $jobs.length > 0) {
						selectedJobId = $jobs[$jobs.length - 1]
					}
					$jobsDrawerOpen = true
				}}
				unifiedSize="md"
				variant={hasErrors ? 'accent' : 'default'}
				btnClasses={'relative'}
				destructive={hasErrors}
				startIcon={{ icon: Bug }}
			>
				<div class="flex flex-row gap-1 items-center">
					<div>Debug runs</div>
					<div class="text-2xs {hasErrors ? '' : 'text-primary'}"
						>({$jobs?.length > 99 ? '99+' : ($jobs?.length ?? 0)})</div
					>
					{#if hasErrors}
						<Button
							size="xs"
							btnClasses="-my-2 !px-1 !py-0"
							variant="default"
							on:click={() => errorByComponent.set({})}
							><BellOff size={12} />
						</Button>
					{/if}
				</div>
			</Button>
		</div>
		<!-- Draws nothing in the bar: it is the export drawer, opened from the menu's Export
		     entries through this instance. The preview switch keeps its place on a phone, since
		     seeing the app is most of what an app editor is good for on one. -->
		<AppExportButton bind:this={appExport} />
		<PreviewToggle loading={loading.save} iconOnly={phoneTopbar} />
		{#if !phoneTopbar}
			<div class="relative shrink-0">
				{#if settingsHaveErrors}
					<span
						class="absolute inline-flex rounded-full bg-red-600 h-2 w-2 z-50 -right-0.5 -top-0.5"
					></span>
				{/if}
				<Button
					variant="default"
					unifiedSize="md"
					startIcon={{ icon: Settings }}
					iconOnly={compactTopbar}
					title="Settings"
					on:click={() => openSettings('metadata')}
				>
					Settings
				</Button>
			</div>
			{#if ownsPageHeader}
				<ShareButton
					deployed={!newApp}
					iconOnly={compactTopbar}
					onclick={() => openShare('editor_header')}
				/>
			{/if}
		{/if}
		<DeployButton
			loadingSave={loading.save}
			iconOnly={phoneTopbar}
			unifiedSize={phoneTopbar ? 'sm' : 'md'}
			on:save={({ detail }) => deploy(detail)}
			dropdownItems={$appPath != ''
				? () => [
						{
							label: 'Fork',
							onClick: () => {
								window.open(`/apps/add?template=${$appPath}`)
							}
						},
						...(!isCloudHosted() && editInForkAllowed($workspaceStore, $userWorkspaces)
							? [
									{
										label: editInForkLabel($workspaceStore, $userWorkspaces),
										onClick: () => {
											openEditInFork('app', $appPath, $workspaceStore)
										}
									}
								]
							: [])
					]
				: undefined}
		/>
	</div>
{/snippet}
