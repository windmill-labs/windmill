<script lang="ts">
	import { Drawer, DrawerContent, UndoRedo } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Toggle from '$lib/components/Toggle.svelte'
	import { AppService, DraftService, type Policy } from '$lib/gen'
	import { redo, undo } from '$lib/history.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import type { Item } from '$lib/utils'
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
		Save,
		Smartphone,
		FileClock,
		Sun,
		Moon,
		SunMoon,
		Zap,
		Globe
	} from 'lucide-svelte'
	import { getContext, untrack } from 'svelte'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		type Value,
		replaceFalseWithUndefined
	} from '../../../utils'
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

	import Summary from '$lib/components/Summary.svelte'
	import HideButton from './settingsPanel/HideButton.svelte'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import AppJobsDrawer from './AppJobsDrawer.svelte'
	import LazyModePanel from './contextPanel/LazyModePanel.svelte'
	import type { DiffDrawerI } from '$lib/components/diff_drawer'
	import AppEditorHeaderDeploy from './AppEditorHeaderDeploy.svelte'
	import AppEditorHeaderDeployInitialDraft from './AppEditorHeaderDeployInitialDraft.svelte'
	import { computeSecretUrl } from './appDeploy.svelte'
	import { updatePolicy } from './appPolicy'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { buildForkEditUrl } from '$lib/utils/editInFork'

	interface Props {
		policy: Policy
		fromHub?: boolean
		diffDrawer?: DiffDrawerI | undefined
		savedApp?:
			| {
					value: App
					draft?: any
					path: string
					summary: string
					policy: any
					draft_only?: boolean
					custom_path?: string
			  }
			| undefined
		version?: number | undefined
		leftPanelHidden?: boolean
		rightPanelHidden?: boolean
		bottomPanelHidden?: boolean
		newApp: boolean
		newPath?: string
		unsavedConfirmationModal?: import('svelte').Snippet<[any]>
		onSavedNewAppPath?: (path: string) => void
		onShowRightPanel?: () => void
		onShowLeftPanel?: () => void
		onShowBottomPanel?: () => void
		onHideRightPanel?: () => void
		onHideLeftPanel?: () => void
		onHideBottomPanel?: () => void
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
		unsavedConfirmationModal,
		onSavedNewAppPath,
		onShowLeftPanel,
		onShowRightPanel,
		onShowBottomPanel,
		onHideLeftPanel,
		onHideRightPanel,
		onHideBottomPanel
	}: Props = $props()

	let newEditedPath = $state('')
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

	const { history, jobsDrawerOpen, refreshComponents } =
		getContext<AppEditorContext>('AppEditorContext')

	const loading = $state({
		publish: false,
		save: false,
		saveDraft: false
	})

	let selectedJobId: string | undefined = $state(undefined)

	let pathError: string = $state('')
	let appExport: AppExportButton | undefined = $state()

	let draftDrawerOpen = $state(false)
	let saveDrawerOpen = $state(false)
	let inputsDrawerOpen = $state(fromHub)
	let historyBrowserDrawerOpen = $state(false)
	let debugAppDrawerOpen = $state(false)
	let lazyDrawerOpen = $state(false)
	let deploymentMsg = $state('')
	let preserveOnBehalfOf = $state(false)

	function closeSaveDrawer() {
		saveDrawerOpen = false
	}

	function closeDraftDrawer() {
		draftDrawerOpen = false
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
					preserve_on_behalf_of: preserveOnBehalfOf || undefined
				}
			})
			savedApp = {
				summary: $summary,
				value: structuredClone($state.snapshot($app)),
				path: path,
				policy: policy,
				custom_path: customPath
			}
			closeSaveDrawer()
			sendUserToast('App deployed successfully')
			try {
				localStorage.removeItem(`app-${path}`)
			} catch (e) {
				console.error('error interacting with local storage', e)
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
							path: newEditedPath || savedApp.draft?.path || savedApp.path,
							policy,
							custom_path: customPath
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
				preserve_on_behalf_of: preserveOnBehalfOf || undefined
			}
		})
		savedApp = {
			summary: $summary,
			value: structuredClone($state.snapshot($app)),
			path: npath,
			policy,
			custom_path: customPath
		}
		const appHistory = await AppService.getAppHistoryByPath({
			workspace: $workspaceStore!,
			path: npath
		})
		version = appHistory[0]?.version

		closeSaveDrawer()
		sendUserToast('App deployed successfully')
		if ($appPath !== npath) {
			try {
				localStorage.removeItem(`app-${appPath}`)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
			onSavedNewAppPath?.(npath)
		}
	}

	async function setPublishState() {
		policy = await updatePolicy($app, policy)
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: $appPath,
			requestBody: { policy }
		})
		if (policy.execution_mode == 'anonymous') {
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

	async function saveInitialDraft() {
		policy = await updatePolicy($app, policy)
		try {
			await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: $app,
					path: newEditedPath,
					summary: $summary,
					policy,
					draft_only: true,
					custom_path: customPath
				}
			})
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: newEditedPath,
					typ: 'app',
					value: {
						value: $app,
						path: newEditedPath,
						summary: $summary,
						policy,
						custom_path: customPath
					}
				}
			})
			savedApp = {
				summary: $summary,
				value: structuredClone($state.snapshot($app)),
				path: newEditedPath,
				policy,
				draft_only: true,
				draft: {
					summary: $summary,
					value: structuredClone($state.snapshot($app)),
					path: newEditedPath,
					policy,
					custom_path: customPath
				},
				custom_path: customPath
			}

			draftDrawerOpen = false
			onSavedNewAppPath?.(newEditedPath)
		} catch (e) {
			sendUserToast('Error saving initial draft', e)
		}
		draftDrawerOpen = false
	}

	async function saveDraft(forceSave = false) {
		if (newApp) {
			// initial draft
			draftDrawerOpen = true
			return
		}
		if (!savedApp) {
			return
		}
		const draftOrDeployed = cleanValueProperties(savedApp.draft || savedApp)
		const current = cleanValueProperties({
			summary: $summary,
			value: $app,
			path: newEditedPath || savedApp.draft?.path || savedApp.path,
			policy
		})
		if (!forceSave && orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(current)) {
			sendUserToast('No changes detected, ignoring', false, [
				{
					label: 'Save anyway',
					callback: () => {
						saveDraft(true)
					}
				}
			])
			return
		}
		loading.saveDraft = true
		try {
			policy = await updatePolicy($app, policy)
			let path = $appPath
			if (savedApp.draft_only) {
				await AppService.deleteApp({
					workspace: $workspaceStore!,
					path: path
				})
				await AppService.createApp({
					workspace: $workspaceStore!,
					requestBody: {
						value: $app!,
						summary: $summary,
						policy,
						path: newEditedPath || path,
						draft_only: true,
						custom_path: customPath
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: savedApp.draft_only ? newEditedPath || path : path,
					typ: 'app',
					value: {
						value: $app!,
						summary: $summary,
						policy,
						path: newEditedPath || path
					}
				}
			})

			savedApp = {
				...(savedApp?.draft_only
					? {
							summary: $summary,
							value: structuredClone($state.snapshot($app)),
							path: savedApp.draft_only ? newEditedPath || path : path,
							policy,
							draft_only: true,
							custom_path: customPath
						}
					: savedApp),
				draft: {
					summary: $summary,
					value: structuredClone($state.snapshot($app)),
					path: newEditedPath || path,
					policy,
					custom_path: customPath
				}
			}

			sendUserToast('Draft saved')
			try {
				localStorage.removeItem(`app-${path}`)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
			loading.saveDraft = false
			if (newApp || savedApp.draft_only) {
				onSavedNewAppPath?.(newEditedPath || path)
			}
		} catch (e) {
			loading.saveDraft = false
			throw e
		}
	}

	let onLatest = $state(true)
	async function compareVersions() {
		if (version === undefined) {
			return
		}
		try {
			const appVersion = await AppService.getAppLatestVersion({
				workspace: $workspaceStore!,
				path: $appPath
			})
			onLatest = appVersion?.version === undefined || version === appVersion?.version
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

		switch (event.key) {
			case 'Z':
				if (event.ctrlKey || event.metaKey) {
					const napp = redo(history)
					for (const key in napp) {
						$app[key] = napp[key]
					}
					event.preventDefault()
				}
				break
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					const napp = undo(history, $app)
					for (const key in napp) {
						$app[key] = napp[key]
					}
					event.preventDefault()
				}
				break
			case 's':
				if (event.ctrlKey || event.metaKey) {
					saveDraft()
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

	let moreItems = $derived([
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
			disabled: !savedApp || savedApp.draft_only
		},
		{
			displayName: 'Diff',
			icon: DiffIcon,
			action: async () => {
				if (!savedApp) {
					return
				}

				// deployedValue should be syncronized when we open Diff
				await syncWithDeployed()

				diffDrawer?.openDrawer()
				diffDrawer?.setDiff({
					mode: 'normal',
					deployed: deployedValue ?? savedApp,
					draft: savedApp.draft,
					current: {
						summary: $summary,
						value: $app,
						path: newEditedPath || savedApp.draft?.path || savedApp.path,
						policy,
						custom_path: customPath
					}
				})
			},
			disabled: !savedApp
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

	let customPath = $state(savedApp?.custom_path)

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

{#if unsavedConfirmationModal}
	{@render unsavedConfirmationModal?.({
		diffDrawer,
		additionalExitAction: () => {
			setTheme(priorDarkMode)
		},
		getInitialAndModifiedValues: () => ({
			savedValue: savedApp,
			modifiedValue: {
				summary: $summary,
				value: $app,
				path: newEditedPath || savedApp?.draft?.path || savedApp?.path,
				policy,
				custom_path: customPath
			}
		})
	})}
{/if}
<DeployOverrideConfirmationModal
	{deployedBy}
	{confirmCallback}
	bind:open
	{diffDrawer}
	bind:deployedValue
	currentValue={{
		summary: $summary,
		value: $app,
		path: newEditedPath || savedApp?.draft?.path || savedApp?.path,
		policy,
		custom_path: customPath
	}}
/>

{#if $appPath == ''}
	<Drawer bind:open={draftDrawerOpen} size="800px">
		<DrawerContent title="Initial draft save" on:close={() => closeDraftDrawer()}>
			{#snippet actions()}
				<div>
					<Button
						startIcon={{ icon: Save }}
						disabled={pathError != ''}
						on:click={() => saveInitialDraft()}
						unifiedSize="md"
						variant="accent"
					>
						Save initial draft
					</Button>
				</div>
			{/snippet}

			<AppEditorHeaderDeployInitialDraft
				bind:summary={$summary}
				bind:appPath={$appPath}
				bind:pathError
				bind:newEditedPath
			/>
		</DrawerContent>
	</Drawer>
{/if}

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
					disabled={!savedApp || savedApp.draft_only}
					on:click={async () => {
						if (!savedApp) {
							return
						}
						// deployedValue should be syncronized when we open Diff
						await syncWithDeployed()

						saveDrawerOpen = false
						diffDrawer?.openDrawer()
						diffDrawer?.setDiff({
							mode: 'normal',
							deployed: deployedValue ?? savedApp,
							draft: savedApp.draft,
							current: {
								summary: $summary,
								value: $app,
								path: newEditedPath || savedApp.draft?.path || savedApp.path,
								policy,
								custom_path: customPath
							},
							button: {
								text: 'Looks good, deploy',
								onClick: () => {
									if ($appPath == '') {
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
						if ($appPath == '') {
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
		<DeploymentHistory on:restore appPath={$appPath} />
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
	class="border-b flex flex-row justify-between py-1 gap-2 gap-y-2 px-2 items-center overflow-y-visible overflow-x-auto"
>
	<div class="flex flex-row gap-2 items-center">
		<Summary bind:value={$summary} />
		<div class="flex gap-2">
			<UndoRedo
				undoProps={{ disabled: $history?.index === 0 }}
				redoProps={{ disabled: $history && $history?.index === $history.history.length - 1 }}
				on:undo={() => {
					const napp = undo(history, $app)
					for (const key in napp) {
						$app[key] = napp[key]
					}
				}}
				on:redo={() => {
					const napp = redo(history)
					for (const key in napp) {
						$app[key] = napp[key]
					}
				}}
			/>

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
	</div>

	{#if $mode !== 'preview'}
		<div class="flex gap-1">
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
		<Awareness />
	{/if}
	<div class="flex flex-row gap-2 justify-end items-center overflow-visible">
		<Dropdown items={moreItems} />
		<AppEditorTutorial bind:this={appEditorTutorial} />

		<div class="hidden md:inline relative overflow-visible shrink-0">
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
			on:click={() => saveDraft()}
			unifiedSize="md"
			disabled={!newApp && !savedApp}
			shortCut={{ key: 'S' }}
		>
			Draft
		</Button>
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
						...(!isRuleActive('DisableWorkspaceForking')
							? [
									{
										label: 'Edit in workspace fork',
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
