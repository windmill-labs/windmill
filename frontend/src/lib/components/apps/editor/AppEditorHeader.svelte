<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Alert, Drawer, DrawerContent, UndoRedo } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Path from '$lib/components/Path.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { AppService, DraftService, type Policy } from '$lib/gen'
	import { redo, undo } from '$lib/history'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
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
		Loader2,
		Save,
		Smartphone,
		FileClock,
		Sun,
		Moon,
		SunMoon,
		Zap
	} from 'lucide-svelte'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		type Value,
		replaceFalseWithUndefined
	} from '../../../utils'
	import type { AppInput, Runnable } from '../inputType'
	import type { App, AppEditorContext, AppViewerContext } from '../types'
	import { BG_PREFIX, allItems, toStatic } from '../utils'
	import AppExportButton from './AppExportButton.svelte'
	import AppInputs from './AppInputs.svelte'
	import type { AppComponent } from './component/components'
	import PreviewToggle from './PreviewToggle.svelte'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from './DeploymentHistory.svelte'
	import Awareness from '$lib/components/Awareness.svelte'
	import { secondaryMenuLeftStore, secondaryMenuRightStore } from './settingsPanel/secondaryMenu'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import AppEditorTutorial from './AppEditorTutorial.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import AppReportsDrawer from './AppReportsDrawer.svelte'
	import { type ColumnDef, getPrimaryKeys } from '../components/display/dbtable/utils'
	import DebugPanel from './contextPanel/DebugPanel.svelte'
	import { getCountInput } from '../components/display/dbtable/queries/count'
	import { getSelectInput } from '../components/display/dbtable/queries/select'
	import { getInsertInput } from '../components/display/dbtable/queries/insert'
	import { getUpdateInput } from '../components/display/dbtable/queries/update'
	import { getDeleteInput } from '../components/display/dbtable/queries/delete'
	import { collectOneOfFields } from './appUtils'
	import Summary from '$lib/components/Summary.svelte'
	import HideButton from './settingsPanel/HideButton.svelte'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'
	import {
		computeS3FileInputPolicy,
		computeWorkspaceS3FileInputPolicy,
		computeS3ImageViewerPolicy
	} from './appUtilsS3'
	import { isCloudHosted } from '$lib/cloud'
	import { base } from '$lib/base'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import AppJobsDrawer from './AppJobsDrawer.svelte'
	import { collectStaticFields, type TriggerableV2 } from './commonAppUtils'
	import LazyModePanel from './contextPanel/LazyModePanel.svelte'
	import { Sha256 } from '@aws-crypto/sha256-js'

	async function hash(message) {
		try {
			const msgUint8 = new TextEncoder().encode(message) // encode as (utf-8) Uint8Array
			const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8) // hash the message
			const hashArray = Array.from(new Uint8Array(hashBuffer)) // convert buffer to byte array
			const hashHex = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
			return hashHex
		} catch {
			//subtle not available, trying pure js
			const hash = new Sha256()
			hash.update(message ?? '')
			const result = Array.from(await hash.digest())
			const hex = result.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
			return hex
		}
	}

	interface Props {
		policy: Policy
		fromHub?: boolean
		diffDrawer?: DiffDrawer | undefined
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
		unsavedConfirmationModal
	}: Props = $props()

	let newEditedPath = $state('')
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

	const { history, jobsDrawerOpen, refreshComponents } =
		getContext<AppEditorContext>('AppEditorContext')

	const loading = $state({
		publish: false,
		save: false,
		saveDraft: false
	})

	let selectedJobId: string | undefined = $state(undefined)

	let pathError: string | undefined = $state(undefined)
	let appExport: AppExportButton | undefined = $state()

	let draftDrawerOpen = $state(false)
	let saveDrawerOpen = $state(false)
	let inputsDrawerOpen = $state(fromHub)
	let historyBrowserDrawerOpen = $state(false)
	let debugAppDrawerOpen = $state(false)
	let lazyDrawerOpen = $state(false)
	let deploymentMsg: string | undefined = $state(undefined)

	function closeSaveDrawer() {
		saveDrawerOpen = false
	}

	function closeDraftDrawer() {
		draftDrawerOpen = false
	}

	async function computeTriggerables() {
		const items = allItems($app.grid, $app.subgrids)

		console.debug('items', items)

		const allTriggers: ([string, TriggerableV2] | undefined)[] = (await Promise.all(
			items
				.flatMap((x) => {
					let c = x.data as AppComponent
					let r: { input: AppInput | undefined; id: string }[] = [
						{ input: c.componentInput, id: x.id }
					]
					if (c.type === 'tablecomponent') {
						r.push(...c.actionButtons.map((x) => ({ input: x.componentInput, id: x.id })))
					}
					if (
						(c.type === 'aggridcomponent' ||
							c.type === 'aggridcomponentee' ||
							c.type === 'dbexplorercomponent' ||
							c.type === 'aggridinfinitecomponent' ||
							c.type === 'aggridinfinitecomponentee') &&
						Array.isArray(c.actions)
					) {
						r.push(...c.actions.map((x) => ({ input: x.componentInput, id: x.id })))
					}
					if (c.type === 'menucomponent') {
						r.push(...c.menuItems.map((x) => ({ input: x.componentInput, id: x.id })))
					}
					if (c.type === 'dbexplorercomponent') {
						let nr: { id: string; input: AppInput }[] = []
						let config = c.configuration as any

						const dbType = config?.type?.selected
						let pg = config?.type?.configuration?.[dbType]

						if (pg && dbType) {
							const { table, resource } = pg
							const tableValue = table.value
							const resourceValue = resource.value
							const columnDefs = (c.configuration.columnDefs as any).value as ColumnDef[]
							const whereClause = (c.configuration.whereClause as any).value as unknown as
								| string
								| undefined
							if (tableValue && resourceValue && columnDefs) {
								r.push({
									input: getSelectInput(resourceValue, tableValue, columnDefs, whereClause, dbType),
									id: x.id
								})

								r.push({
									input: getCountInput(resourceValue, tableValue, dbType, columnDefs, whereClause),
									id: x.id + '_count'
								})

								r.push({
									input: getInsertInput(tableValue, columnDefs, resourceValue, dbType),
									id: x.id + '_insert'
								})

								let primaryColumns = getPrimaryKeys(columnDefs)
								let columns = columnDefs?.filter((x) => primaryColumns.includes(x.field))

								r.push({
									input: getDeleteInput(resourceValue, tableValue, columns, dbType),
									id: x.id + '_delete'
								})

								columnDefs
									.filter((col) => col.editable || config.allEditable.value)
									.forEach((column) => {
										r.push({
											input: getUpdateInput(resourceValue, tableValue, column, columns, dbType),
											id: x.id + '_update'
										})
									})
							}
						}
						r.push(...nr)
					}

					const processed = r
						.filter((x) => x.input)
						.map(async (o) => {
							if (o.input?.type == 'runnable') {
								return await processRunnable(o.id, o.input.runnable, o.input.fields)
							}
						})

					return processed as Promise<[string, TriggerableV2] | undefined>[]
				})
				.concat(
					Object.values($app.hiddenInlineScripts ?? {}).map(async (v, i) => {
						return await processRunnable(BG_PREFIX + i, v, v.fields)
					}) as Promise<[string, TriggerableV2] | undefined>[]
				)
		)) as ([string, TriggerableV2] | undefined)[]

		delete policy.triggerables
		const ntriggerables: Record<string, TriggerableV2> = Object.fromEntries(
			allTriggers.filter(Boolean) as [string, TriggerableV2][]
		)
		policy.triggerables_v2 = ntriggerables

		const s3_inputs = items
			.filter((x) => (x.data as AppComponent).type === 's3fileinputcomponent')
			.map((x) => {
				const c = x.data as AppComponent
				const config = c.configuration as any
				return computeS3FileInputPolicy(config?.type?.configuration?.s3, $app)
			})
			.filter(Boolean) as {
			allowed_resources: string[]
			allow_user_resources: boolean
			file_key_regex: string
		}[]

		if (
			items.findIndex((x) => {
				const c = x.data as AppComponent
				if (c.type === 'schemaformcomponent') {
					return (
						Object.values((c.componentInput as any)?.value?.properties ?? {}).findIndex(
							(p: any) => p?.type === 'object' && p?.format === 'resource-s3_object'
						) !== -1
					)
				} else if (c.type === 'formbuttoncomponent' || c.type === 'formcomponent') {
					return (
						Object.values((c.componentInput as any)?.fields ?? {}).findIndex(
							(p: any) => p?.fieldType === 'object' && p?.format === 'resource-s3_object'
						) !== -1
					)
				} else {
					return false
				}
			}) !== -1
		) {
			s3_inputs.push(computeWorkspaceS3FileInputPolicy())
		}

		policy.s3_inputs = s3_inputs

		const s3FileKeys = items
			.filter((x) => (x.data as AppComponent).type === 'imagecomponent')
			.map((x) => {
				const c = x.data as AppComponent
				const config = c.configuration
				return computeS3ImageViewerPolicy(config)
			})
			.filter(Boolean) as { s3_path: string; storage?: string | undefined }[]

		policy.allowed_s3_keys = s3FileKeys
	}

	async function processRunnable(
		id: string,
		runnable: Runnable,
		fields: Record<string, any>
	): Promise<[string, TriggerableV2] | undefined> {
		const staticInputs = collectStaticFields(fields)
		const oneOfInputs = collectOneOfFields(fields, $app)
		const allowUserResources: string[] = Object.entries(fields)
			.map(([k, v]) => {
				return v['allowUserResources'] ? k : undefined
			})
			.filter(Boolean) as string[]

		if (runnable?.type == 'runnableByName') {
			let hex = await hash(runnable.inlineScript?.content)
			console.log('hex', hex, id)
			return [
				`${id}:rawscript/${hex}`,
				{
					static_inputs: staticInputs,
					one_of_inputs: oneOfInputs,
					allow_user_resources: allowUserResources
				}
			]
		} else if (runnable?.type == 'runnableByPath') {
			let prefix = runnable.runType !== 'hubscript' ? runnable.runType : 'script'
			return [
				`${id}:${prefix}/${runnable.path}`,
				{
					static_inputs: staticInputs,
					one_of_inputs: oneOfInputs,
					allow_user_resources: allowUserResources
				}
			]
		}
	}

	async function createApp(path: string) {
		await computeTriggerables()
		try {
			await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: $app,
					path,
					summary: $summary,
					policy,
					deployment_message: deploymentMsg,
					custom_path: customPath
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
			dispatch('savedNewAppPath', path)
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
		await computeTriggerables()
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
					$userStore?.is_admin || $userStore?.is_super_admin ? (customPath ?? '') : undefined
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
			dispatch('savedNewAppPath', npath)
		}
	}

	let secretUrl: string | undefined = $state(undefined)

	async function getSecretUrl() {
		secretUrl = await AppService.getPublicSecretOfApp({
			workspace: $workspaceStore!,
			path: $appPath
		})
	}

	async function setPublishState() {
		await computeTriggerables()
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
		await computeTriggerables()
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
			dispatch('savedNewAppPath', newEditedPath)
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
			await computeTriggerables()
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
				dispatch('savedNewAppPath', newEditedPath || path)
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
					$app = redo(history)
					event.preventDefault()
				}
				break
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					$app = undo(history, $app)

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

	let dirtyPath = $state(false)
	let path: Path | undefined = $state(undefined)

	let moreItems = [
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
		// {
		// 	displayName: 'Publish to Hub',
		// 	icon: faGlobe,
		// 	action: () => {
		// 		const url = appToHubUrl(toStatic($app, $staticExporter, $summary, $hubBaseUrlStore))
		// 		window.open(url.toString(), '_blank')
		// 	}
		// },
		{
			displayName: 'Hub compatible JSON',
			icon: FileUp,
			action: () => {
				appExport?.open(toStatic($app, $staticExporter, $summary).app)
			}
		},
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
		}
	]

	let appEditorTutorial: AppEditorTutorial | undefined = $state(undefined)

	export function toggleTutorial() {
		appEditorTutorial?.toggleTutorial()
	}

	let appReportingDrawerOpen = $state(false)

	export function openTroubleshootPanel() {
		debugAppDrawerOpen = true
	}

	const dispatch = createEventDispatcher()

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
	}

	let priorDarkMode = document.documentElement.classList.contains('dark')
	setTheme($app?.darkMode)

	let customPath = $state(savedApp?.custom_path)
	let dirtyCustomPath = $state(false)
	let customPathError = $state('')
	async function appExists(customPath: string) {
		return await AppService.customPathExists({
			workspace: $workspaceStore!,
			customPath
		})
	}
	let validateTimeout: NodeJS.Timeout | undefined = undefined
	async function validateCustomPath(customPath: string): Promise<void> {
		customPathError = ''
		if (validateTimeout) {
			clearTimeout(validateTimeout)
		}
		validateTimeout = setTimeout(async () => {
			if (!/^[\w-]+(\/[\w-]+)*$/.test(customPath)) {
				customPathError = 'Invalid path'
			} else if (customPath !== savedApp?.custom_path && (await appExists(customPath))) {
				customPathError = 'Path already taken'
			} else {
				customPathError = ''
			}
			validateTimeout = undefined
		}, 500)
	}
	$effect(() => {
		if ($openDebugRun == undefined) {
			$openDebugRun = (jobId: string) => {
				$jobsDrawerOpen = true
				selectedJobId = jobId
			}
		}
	})
	$effect(() => {
		$appPath && $appPath != '' && secretUrl == undefined && untrack(() => getSecretUrl())
	})
	$effect(() => {
		saveDrawerOpen && untrack(() => compareVersions())
	})
	let hasErrors = $derived(Object.keys($errorByComponent).length > 0)
	let fullCustomUrl = $derived(
		`${window.location.origin}${base}/a/${
			isCloudHosted() ? $workspaceStore + '/' : ''
		}${customPath}`
	)
	$effect(() => {
		;[customPath]
		untrack(() => customPath !== undefined && validateCustomPath(customPath))
	})
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
	bind:deployedBy
	bind:confirmCallback
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
			<Alert bgClass="mb-4" title="Require path" type="info">
				Choose a path to save the initial draft of the app.
			</Alert>
			<h3>Summary</h3>
			<div class="w-full pt-2">
				<!-- svelte-ignore a11y_autofocus -->
				<input
					autofocus
					type="text"
					placeholder="App summary"
					class="text-sm w-full font-semibold"
					onkeydown={stopPropagation(bubble('keydown'))}
					bind:value={$summary}
					onkeyup={() => {
						if ($appPath == '' && $summary?.length > 0 && !dirtyPath) {
							path?.setName(
								$summary
									.toLowerCase()
									.replace(/[^a-z0-9_]/g, '_')
									.replace(/-+/g, '_')
									.replace(/^-|-$/g, '')
							)
						}
					}}
				/>
			</div>
			<div class="py-2"></div>
			<Path
				autofocus={false}
				bind:this={path}
				bind:error={pathError}
				bind:path={newEditedPath}
				bind:dirty={dirtyPath}
				initialPath=""
				namePlaceholder="app"
				kind="app"
			/>
			<div class="py-4"></div>

			{#snippet actions()}
				<div>
					<Button
						startIcon={{ icon: Save }}
						disabled={pathError != ''}
						on:click={() => saveInitialDraft()}
					>
						Save initial draft
					</Button>
				</div>
			{/snippet}
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
		{#if !onLatest}
			<Alert title="You're not on the latest app version. " type="warning">
				By deploying, you may overwrite changes made by other users. Press 'Deploy' to see diff.
			</Alert>
			<div class="py-2"></div>
		{/if}
		<span class="text-secondary text-sm font-bold">Summary</span>
		<div class="w-full pt-2">
			<!-- svelte-ignore a11y_autofocus -->
			<input
				autofocus
				type="text"
				placeholder="App summary"
				class="text-sm w-full"
				bind:value={$summary}
				onkeydown={stopPropagation(bubble('keydown'))}
				onkeyup={() => {
					if ($appPath == '' && $summary?.length > 0 && !dirtyPath) {
						path?.setName(
							$summary
								.toLowerCase()
								.replace(/[^a-z0-9_]/g, '_')
								.replace(/-+/g, '_')
								.replace(/^-|-$/g, '')
						)
					}
				}}
			/>
		</div>
		<div class="py-4"></div>
		<span class="text-secondary text-sm font-bold">Deployment message</span>
		<div class="w-full pt-2">
			<!-- svelte-ignore a11y_autofocus -->
			<input
				type="text"
				placeholder="Optional deployment message"
				class="text-sm w-full"
				bind:value={deploymentMsg}
			/>
		</div>
		<div class="py-4"></div>
		<span class="text-secondary text-sm font-bold">Path</span>
		<Path
			bind:this={path}
			bind:dirty={dirtyPath}
			bind:error={pathError}
			bind:path={newEditedPath}
			initialPath={newPath}
			namePlaceholder="app"
			kind="app"
			autofocus={false}
		/>

		{#snippet actions()}
			<div class="flex flex-row gap-4">
				<Button
					variant="border"
					color="light"
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
		<div class="py-2"></div>
		{#if $appPath == ''}
			<Alert title="Require saving" type="error">
				Save this app once before you can publish it
			</Alert>
		{:else}
			<Alert title="App executed on behalf of you">
				A viewer of the app will execute the runnables of the app on behalf of the publisher (you)
				<Tooltip>
					It ensures that all required resources/runnable visible for publisher but not for viewer
					at time of creating the app would prevent the execution of the app. To guarantee tight
					security, a policy is computed at time of deployment of the app which only allow the
					scripts/flows referred to in the app to be called on behalf of. Furthermore, static
					parameters are not overridable. Hence, users will only be able to use the app as intended
					by the publisher without risk for leaking resources not used in the app.
				</Tooltip>
			</Alert>

			<div class="mt-10"></div>

			<h2>Public URL</h2>
			<div class="mt-4"></div>

			<div class="flex gap-2 items-center">
				<Toggle
					options={{
						left: `Require login and read-access`,
						right: `No login required`
					}}
					checked={policy.execution_mode == 'anonymous'}
					on:change={(e) => {
						policy.execution_mode = e.detail ? 'anonymous' : 'publisher'
						setPublishState()
					}}
				/>
			</div>

			<div class="my-6 box">
				<div class="text-secondary">
					<div>Public URL</div>
				</div>
				{#if secretUrl}
					{@const href = `${window.location.origin}${base}/public/${$workspaceStore}/${secretUrl}`}
					<ClipboardPanel content={href} size="md" />
				{:else}<Loader2 class="animate-spin" />
				{/if}
				<div class="text-xs text-secondary mt-1">
					Share this url directly or embed it using an iframe (if requiring login, top-level domain
					of embedding app must be the same as the one of Windmill)
				</div>

				<div class="mt-4">
					{#if !$enterpriseLicense}
						<Alert title="EE Only" type="warning" size="xs">
							Custom path is an enterprise only feature.
						</Alert>
						<div class="mb-2"></div>
					{:else if !($userStore?.is_admin || $userStore?.is_super_admin)}
						<Alert type="warning" title="Admin only" size="xs">
							Custom path can only be set by workspace admins
						</Alert>
						<div class="mb-2"></div>
					{/if}
					<Toggle
						on:change={({ detail }) => {
							customPath = detail ? '' : undefined
							if (customPath === undefined) {
								customPathError = ''
							}
						}}
						checked={customPath !== undefined}
						options={{
							right: 'Use a custom URL'
						}}
						disabled={!$enterpriseLicense || !($userStore?.is_admin || $userStore?.is_super_admin)}
					/>

					{#if customPath !== undefined}
						<div class="text-secondary text-sm flex items-center gap-1 w-full justify-between">
							<div>Custom path</div>
						</div>
						<input
							disabled={!($userStore?.is_admin || $userStore?.is_super_admin)}
							type="text"
							autocomplete="off"
							bind:value={customPath}
							class={customPathError === ''
								? ''
								: 'border border-red-700 bg-red-100 border-opacity-30 focus:border-red-700 focus:border-opacity-30 focus-visible:ring-red-700 focus-visible:ring-opacity-25 focus-visible:border-red-700'}
							oninput={() => {
								dirtyCustomPath = true
							}}
						/>
						<div class="text-secondary text-sm flex items-center gap-1 mt-2 w-full justify-between">
							<div>Custom public URL</div>
						</div>
						<ClipboardPanel content={fullCustomUrl} size="md" />

						<div class="text-red-600 dark:text-red-400 text-2xs mt-1.5"
							>{dirtyCustomPath ? customPathError : ''}
						</div>
					{/if}
				</div>
			</div>
			<Alert type="info" title="Only latest deployed app is publicly available">
				You will still need to deploy the app to make visible the latest changes
			</Alert>

			<a
				href="https://www.windmill.dev/docs/advanced/external_auth_with_jwt#embed-public-apps-using-your-own-authentification"
				class="mt-4 text-2xs">Embed this app in your own product to be used by your own users</a
			>
		{/if}
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
					$app = undo(history, $app)
				}}
				on:redo={() => {
					$app = redo(history)
				}}
			/>

			{#if $app}
				<ToggleButtonGroup
					class="h-[30px]"
					selected={$app.fullscreen ? 'true' : 'false'}
					on:selected={({ detail }) => {
						$app.fullscreen = detail === 'true'
					}}
				>
					{#snippet children({ item })}
						<ToggleButton
							icon={AlignHorizontalSpaceAround}
							value={'false'}
							tooltip="The max width is 1168px and the content stay centered instead of taking the full page width"
							iconProps={{ size: 16 }}
							{item}
						/>
						<ToggleButton
							tooltip="The width is of the app if the full width of its container"
							icon={Expand}
							value={'true'}
							iconProps={{ size: 16 }}
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>
			{/if}
			{#if $app}
				<ToggleButtonGroup
					class="h-[30px]"
					on:selected={({ detail }) => {
						const theme = detail === 'dark' ? true : detail === 'sun' ? false : undefined
						setTheme(theme)
					}}
					selected={$app.darkMode === undefined ? 'auto' : $app.darkMode ? 'dark' : 'sun'}
				>
					{#snippet children({ item })}
						<ToggleButton
							icon={SunMoon}
							value={'auto'}
							tooltip="The app mode between dark/light is automatic"
							iconProps={{ size: 16 }}
							{item}
						/>
						<ToggleButton
							icon={Sun}
							value={'sun'}
							tooltip="Force light mode"
							iconProps={{ size: 16 }}
							{item}
						/>
						<ToggleButton
							tooltip="Force dark mode"
							icon={Moon}
							value={'dark'}
							iconProps={{ size: 16 }}
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>
			{/if}
			<div class="flex flex-row gap-2">
				<ToggleButtonGroup class="h-[30px]" bind:selected={$breakpoint}>
					{#snippet children({ item })}
						<ToggleButton
							tooltip="Computer View"
							icon={Laptop2}
							value={'lg'}
							iconProps={{ size: 16 }}
							{item}
						/>
						<ToggleButton
							tooltip="Mobile View"
							icon={Smartphone}
							value={'sm'}
							iconProps={{ size: 16 }}
							{item}
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
						dispatch('showLeftPanel')
					} else {
						dispatch('hideLeftPanel')
					}
				}}
			/>
			<HideButton
				hidden={bottomPanelHidden}
				direction="bottom"
				on:click={() => {
					if (bottomPanelHidden) {
						dispatch('showBottomPanel')
					} else {
						dispatch('hideBottomPanel')
					}
				}}
			/>
			<HideButton
				hidden={rightPanelHidden}
				direction="right"
				on:click={() => {
					if (rightPanelHidden) {
						dispatch('showRightPanel')
					} else {
						dispatch('hideRightPanel')
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

		<div class="hidden md:inline relative overflow-visible">
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
				color={hasErrors ? 'red' : 'light'}
				size="xs"
				variant="border"
				btnClasses="relative"
			>
				<div class="flex flex-row gap-1 items-center">
					<Bug size={14} />
					<div>Debug runs</div>
					<div class="text-2xs text-tertiary"
						>({$jobs?.length > 99 ? '99+' : ($jobs?.length ?? 0)})</div
					>
					{#if hasErrors}
						<Button
							size="xs"
							btnClasses="-my-2 !px-1 !py-0"
							color="light"
							variant="border"
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
			loading={loading.save}
			startIcon={{ icon: Save }}
			on:click={() => saveDraft()}
			size="xs"
			disabled={!newApp && !savedApp}
			shortCut={{ key: 'S' }}
		>
			Draft
		</Button>
		<Button
			loading={loading.save}
			startIcon={{ icon: Save }}
			on:click={save}
			size="xs"
			dropdownItems={$appPath != ''
				? () => [
						{
							label: 'Fork',
							onClick: () => {
								window.open(`/apps/add?template=${appPath}`)
							}
						}
					]
				: undefined}
		>
			Deploy
		</Button>
	</div>
</div>
