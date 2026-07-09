<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Toggle from '$lib/components/Toggle.svelte'
	import { AppService, type Policy } from '$lib/gen'
	import { redo, undo } from '$lib/history.svelte'
	import { discardDraftAfterDeploy } from '$lib/userDraftToast'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import {
		enterpriseLicense,
		tutorialsToDo,
		userStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import { isMac, type Item, userPathPrefix } from '$lib/utils'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import { getTutorialIndex } from '$lib/tutorials/config'
	import { random_adj } from '$lib/components/random_positive_adjetive'
	import {
		AlignHorizontalSpaceAround,
		BellOff,
		BookOpen,
		Bug,
		CheckCheck,
		CheckCircle,
		Circle,
		DiffIcon,
		Expand,
		FileJson,
		FileUp,
		FormInput,
		History,
		Laptop2,
		RefreshCw,
		Save,
		Smartphone,
		FileClock,
		Sun,
		Moon,
		SunMoon,
		Undo,
		Redo,
		Zap,
		Globe
	} from 'lucide-svelte'
	import { getContext, untrack } from 'svelte'
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
	import { secondaryMenuLeftStore, secondaryMenuRightStore } from './settingsPanel/secondaryMenu'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import AppEditorTutorial from './AppEditorTutorial.svelte'
	import AppReportsDrawer from './AppReportsDrawer.svelte'
	import DebugPanel from './contextPanel/DebugPanel.svelte'

	import EditorHeader from '$lib/components/EditorHeader.svelte'
	import AutosaveIndicator from '$lib/components/AutosaveIndicator.svelte'
	import { editPathFor } from '$lib/components/workspacePicker'
	import { invalidateWorkspacePaths } from '$lib/components/PathNameAutocomplete.svelte'
	import { beforeNavigate, goto } from '$app/navigation'
	import HideButton from './settingsPanel/HideButton.svelte'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import AppJobsDrawer from './AppJobsDrawer.svelte'
	import LazyModePanel from './contextPanel/LazyModePanel.svelte'
	import type { DiffDrawerI } from '$lib/components/diff_drawer'
	import AppEditorHeaderDeploy from './AppEditorHeaderDeploy.svelte'
	import { computeSecretUrl } from './appDeploy.svelte'
	import { updatePolicy } from './appPolicy'
	import { buildForkEditUrl, editInForkAllowed, editInForkLabel } from '$lib/utils/editInFork'
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
		onRestore
	}: Props = $props()

	/** Mirror of the path the user is editing in the pen popover. Initialized
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
	let customPathError: string = $state('')

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

	// Mirror the summary onto the autosaved App so a draft persists it (the
	// autosave stores the bare App value, which has no summary of its own — it
	// lives in the `app` table column, set only on deploy). Without this the
	// summary is lost when reopening a draft or deploying it from the Review &
	// Deploy page. Parallels `draft_path`.
	$effect(() => {
		const s = $summary
		const a = $app
		if (!a) return
		untrack(() => {
			if (a.summary !== s) a.summary = s
		})
	})

	const { history, jobsDrawerOpen, refreshComponents } =
		getContext<AppEditorContext>('AppEditorContext')

	// Sessions inject an AIChatManager via context; AppEditor skips its
	// UserDraft handle in that case, so the cleanup calls here must skip too
	// (otherwise we'd wipe a non-session tab's autosave at the same path). The
	// session-side equivalent is the View's `onDeploy` →
	// `runtime.syncPreviewWithDeployed`, which discards the fork draft + reloads
	// the preview to the deployed version.
	const inSessionPane = !!getContext('aiChatManager')

	const loading = $state({
		publish: false,
		save: false
	})

	let selectedJobId: string | undefined = $state(undefined)

	let pathError: string = $state('')
	let appExport: AppExportButton | undefined = $state()

	let saveDrawerOpen = $state(false)
	let inputsDrawerOpen = $state(untrack(() => fromHub))
	let historyBrowserDrawerOpen = $state(false)
	let debugAppDrawerOpen = $state(false)
	let lazyDrawerOpen = $state(false)
	let deploymentMsg = $state('')
	let preserveOnBehalfOf = $state(false)

	// Top-bar responsive collapse — container width, not viewport.
	let topbarWidth = $state(0)
	const compactTopbar = $derived(topbarWidth > 0 && topbarWidth < 720)

	function closeSaveDrawer() {
		saveDrawerOpen = false
	}

	async function createApp(path: string) {
		policy = await updatePolicy($app, policy)
		try {
			await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: $app,
					path,
					summary: $summary,
					policy,
					deployment_message: deploymentMsg,
					custom_path: customPath,
					preserve_on_behalf_of: preserveOnBehalfOf || undefined,
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
				policy: policy,
				custom_path: customPath,
				labels: $state.snapshot(labels)
			}
			closeSaveDrawer()
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
		} catch (e) {
			sendUserToast('Error creating app', e)
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
							value: $app,
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

	async function syncWithDeployed() {
		const deployedApp = await AppService.getAppByPath({
			workspace: $workspaceStore!,
			path: $appPath!,
			withStarredInfo: true
		})

		deployedBy = deployedApp.created_by

		// Strip off extra information
		deployedValue = replaceFalseWithUndefined({
			...deployedApp,
			id: undefined,
			created_at: undefined,
			created_by: undefined,
			versions: undefined,
			extra_perms: undefined
		})
	}

	async function updateApp(npath: string) {
		policy = await updatePolicy($app, policy)
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: $appPath!,
			requestBody: {
				value: $app!,
				summary: $summary,
				policy,
				path: npath,
				deployment_message: deploymentMsg,
				// custom_path requires admin so to accept update without it, we need to send as undefined when non-admin (when undefined, it will be ignored)
				// it also means that customPath needs to be set to '' instead of undefined to unset it (when admin)
				custom_path:
					$userStore?.is_admin || $userStore?.is_super_admin ? (customPath ?? '') : undefined,
				preserve_on_behalf_of: preserveOnBehalfOf || undefined,
				labels
			}
		})
		invalidateWorkspacePaths($workspaceStore!)
		savedApp = {
			summary: $summary,
			value: structuredClone($state.snapshot($app)),
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
		// Re-pin the fork base to the just-deployed head: the editor stays open, so a
		// follow-up deploy (or a new edit) would otherwise compare against the now-
		// superseded base and falsely warn. parent_version is in
		// DRAFT_COMPARE_IGNORED_FIELDS, so this write can't spawn a spurious draft.
		if ($app) $app.parent_version = version

		closeSaveDrawer()
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

	async function setPublishState(message?: string) {
		policy = await updatePolicy($app, policy)
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: $appPath,
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
		$secondaryMenuLeftStore.isOpen = false
		$secondaryMenuRightStore.isOpen = false

		saveDrawerOpen = true
		return
	}

	let onLatest = $state(true)
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
		} catch (e) {
			console.error('Error comparing versions', e)
			onLatest = true
		}
	}

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
				if (event.ctrlKey || event.metaKey) {
					event.preventDefault()
					// Flush the pending autosave (also covers the toggle-off parked
					// case); no-op in the AI session pane (no handle there).
					if (!inSessionPane && $workspaceStore && userDraftPath) {
						void UserDraftDbSyncer.flush({
							workspace: $workspaceStore,
							itemKind: 'app',
							path: userDraftPath
						})
					}
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
						},
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
				appExport?.open($app)
			}
		},
		{
			displayName: 'Public URL',
			icon: Globe,
			action: async () => {
				const secretUrl = await AppService.getPublicSecretOfApp({
					workspace: $workspaceStore!,
					path: $appPath
				})
				window.open(computeSecretUrl(secretUrl), '_blank')
			}
		},
		// {
		// 	displayName: 'Publish to Hub',
		// 	icon: faGlobe,
		// 	action: () => {
		// 		const url = appToHubUrl(toStatic($app, $staticExporter, $summary, $hubBaseUrlStore))
		// 		window.open(url.toString(), '_blank')
		// 	}
		// },

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

				// deployedValue should be syncronized when we open Diff
				await syncWithDeployed()

				diffDrawer?.openDrawer()
				diffDrawer?.setDiff({
					mode: 'normal',
					deployed: deployedValue ?? savedApp,
					current: {
						summary: $summary,
						value: $app,
						path: newEditedPath || savedApp.path,
						policy,
						custom_path: customPath,
						labels
					}
				})
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
				appExport?.open(toStatic($app, $staticExporter, $summary).app)
			}
		},
		{
			displayName: 'Tutorials',
			icon: BookOpen,
			separatorTop: true,
			submenuItems: [
				{
					displayName: 'Background runnables',
					action: () => appEditorTutorial?.runTutorialById('backgroundrunnables'),
					icon: $tutorialsToDo.includes(getTutorialIndex('backgroundrunnables'))
						? Circle
						: CheckCircle,
					iconColor: $tutorialsToDo.includes(getTutorialIndex('backgroundrunnables'))
						? undefined
						: 'green'
				},
				{
					displayName: 'Connection',
					action: () => appEditorTutorial?.runTutorialById('connection'),
					icon: $tutorialsToDo.includes(getTutorialIndex('connection')) ? Circle : CheckCircle,
					iconColor: $tutorialsToDo.includes(getTutorialIndex('connection')) ? undefined : 'green'
				},
				{
					displayName: 'Reset tutorials',
					action: () => resetAllTodos(),
					icon: RefreshCw,
					separatorTop: true
				},
				{
					displayName: 'Skip tutorials',
					action: () => skipAllTodos(),
					icon: CheckCheck
				}
			]
		}
	]) as Item[]

	let appEditorTutorial: AppEditorTutorial | undefined = $state(undefined)

	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		appEditorTutorial?.runTutorialById(id, options)
	}

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

	let customPath = $state(savedApp?.custom_path)
	let labels = $state(untrack(() => initialLabels))

	$effect(() => {
		if ($openDebugRun == undefined) {
			$openDebugRun = (jobId: string) => {
				$jobsDrawerOpen = true
				selectedJobId = jobId
			}
		}
	})

	$effect(() => {
		saveDrawerOpen && untrack(() => compareVersions())
	})
	let hasErrors = $derived(Object.keys($errorByComponent).length > 0)
</script>

<svelte:window onkeydown={onKeyDown} />

<DeployOverrideConfirmationModal
	{deployedBy}
	{confirmCallback}
	bind:open
	{diffDrawer}
	bind:deployedValue
	currentValue={{
		summary: $summary,
		value: $app,
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
<Drawer bind:open={saveDrawerOpen} size="800px">
	<DrawerContent title="Deploy" on:close={() => closeSaveDrawer()}>
		{#snippet actions()}
			<div class="flex flex-row gap-4">
				<Button
					variant="accent"
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
							deployed: deployedValue ?? savedApp,
							current: {
								summary: $summary,
								value: $app,
								path: newEditedPath || savedApp.path,
								policy,
								custom_path: customPath,
								labels
							},
							button: {
								text: 'Looks good, deploy',
								onClick: () => {
									if (newApp) {
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
					startIcon={{ icon: Save }}
					disabled={pathError != '' || customPathError != ''}
					on:click={() => {
						// Use `newApp` (set for /apps/add and draft-only paths), not
						// `$appPath` — its `draft_{uuid}` is a poor create signal.
						if (newApp) {
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
			appPath={$appPath}
			{onLatest}
			{savedApp}
			bind:summary={$summary}
			bind:customPath
			bind:deploymentMsg
			bind:customPathError
			bind:pathError
			bind:newEditedPath
			bind:preserveOnBehalfOf
			bind:labels
			hideSecretUrl={false}
		/>
	</DrawerContent>
</Drawer>

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
	</div>

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
	{#if $enterpriseLicense && $appPath != ''}
		<div class="shrink-0">
			<Awareness />
		</div>
	{/if}
	<div class="flex flex-row gap-2 justify-end items-center overflow-visible shrink-0">
		<div class="relative">
			<Dropdown items={moreItems} />
			{#if $tutorialsToDo.includes(getTutorialIndex('backgroundrunnables')) || $tutorialsToDo.includes(getTutorialIndex('connection'))}
				<span
					class="absolute top-0.5 right-0.5 block w-2 h-2 rounded-full bg-surface-accent-primary pointer-events-none"
				></span>
			{/if}
		</div>
		<AppEditorTutorial bind:this={appEditorTutorial} />

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
		<AppExportButton bind:this={appExport} />
		<PreviewToggle loading={loading.save} />
		<Button
			variant="accent"
			loading={loading.save}
			startIcon={{ icon: Save }}
			on:click={save}
			unifiedSize="md"
			dropdownItems={$appPath != ''
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
											window.open(buildForkEditUrl('app', $appPath))
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
