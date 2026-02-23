<script lang="ts">
	import { Badge, Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import UndoRedo from '$lib/components/common/button/UndoRedo.svelte'

	import { AppService, DraftService, type Policy } from '$lib/gen'
	import { rawAppToHubUrl } from '$lib/hub'
	import { enterpriseLicense, hubBaseUrlStore, userStore, workspaceStore } from '$lib/stores'
	import YAML from 'yaml'
	import {
		Bug,
		DiffIcon,
		Download,
		EllipsisVertical,
		FileJson,
		Globe,
		History,
		Pen,
		Save,
		WandSparkles
	} from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import {
		cleanValueProperties,
		orderedJsonStringify,
		type Value,
		replaceFalseWithUndefined,
		defaultIfEmptyString
	} from '../../utils'

	// import {  allItems, toStatic } from '../apps/editor/settingsPanel/utils'
	import AppExportButton from '../apps/editor/AppExportButton.svelte'

	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from '../apps/editor/DeploymentHistory.svelte'
	import Awareness from '$lib/components/Awareness.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'

	import Summary from '$lib/components/Summary.svelte'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'

	import AppJobsDrawer from '../apps/editor/AppJobsDrawer.svelte'
	import type { SavedAndModifiedValue } from '../common/confirmationModal/unsavedTypes'
	import DropdownV2 from '../DropdownV2.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import AppEditorHeaderDeployInitialDraft from '../apps/editor/AppEditorHeaderDeployInitialDraft.svelte'
	import AppEditorHeaderDeploy from '../apps/editor/AppEditorHeaderDeploy.svelte'
	import type { Runnable } from './RawAppInlineScriptRunnable.svelte'
	import { updateRawAppPolicy } from './rawAppPolicy'
	import { aiChatManager } from '../copilot/chat/AIChatManager.svelte'
	import { AIBtnClasses } from '../copilot/chat/AIButtonStyle'
	import type { RawAppData } from './dataTableRefUtils'

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
					draft?: any
					path: string
					summary: string
					policy: any
					draft_only?: boolean
					custom_path?: string
			  }
			| undefined
		version?: number | undefined
		newApp: boolean
		newPath?: string
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
	}

	let {
		summary = $bindable(),
		policy = $bindable(),
		diffDrawer = undefined,
		savedApp = $bindable(undefined),
		version = $bindable(undefined),
		newApp,
		newPath = '',
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
		onRedo = undefined
	}: Props = $props()

	let newEditedPath = $state('')

	let deployedValue: Value | undefined = $state(undefined) // Value to diff against
	let deployedBy: string | undefined = $state(undefined) // Author
	let confirmCallback: () => void = $state(() => {}) // What happens when user clicks `override` in warning
	let open: boolean = $state(false) // Is confirmation modal open

	// const { app, summary, appPath, jobs, jobsById, staticExporter } = getContext('AppViewerContext')

	const loading = $state({
		publish: false,
		save: false,
		saveDraft: false
	})

	let pathError: string = $state('')
	let appExport = $state() as AppExportButton | undefined

	let draftDrawerOpen = $state(false)
	let saveDrawerOpen = $state(false)
	let historyBrowserDrawerOpen = $state(false)
	let publishToHubDrawerOpen = $state(false)
	let publishingToHub = $state(false)
	let deploymentMsg: string | undefined = $state(undefined)

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

	function closeDraftDrawer() {
		draftDrawerOpen = false
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
						custom_path: customPath
					},
					js,
					css
				}
			})
			savedApp = {
				summary: summary,
				value: structuredClone(stateSnapshot(app)),
				path: path,
				policy: policy,
				custom_path: customPath
			}
			closeSaveDrawer()
			sendUserToast('App deployed successfully')
			try {
				localStorage.removeItem(`rawapp-${path}`)
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
				app &&
				orderedJsonStringify(deployedValue) ===
					orderedJsonStringify(
						replaceFalseWithUndefined({
							summary: summary,
							value: app,
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
			path: appPath!,
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
					// custom_path requires admin so to accept update without it, we need to send as undefined when non-admin (when undefined, it will be ignored)
					// it also means that customPath needs to be set to '' instead of undefined to unset it (when admin)
					custom_path:
						$userStore?.is_admin || $userStore?.is_super_admin ? (customPath ?? '') : undefined
				},
				js,
				css
			}
		})
		savedApp = {
			summary: summary,
			value: structuredClone(stateSnapshot(app)),
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
		if (appPath !== npath) {
			try {
				localStorage.removeItem(`rawapp-${appPath}`)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
			dispatch('savedNewAppPath', npath)
		}
	}

	async function setPublishState() {
		await computeTriggerables()
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: appPath,
			requestBody: { policy }
		})
		if (policy.execution_mode == 'anonymous') {
			sendUserToast('App require no login to be accessed')
		} else {
			sendUserToast('App require login and read-access')
		}
	}

	async function save() {
		saveDrawerOpen = true
		return
	}

	async function saveInitialDraft() {
		if (!app) {
			sendUserToast(`App hasn't been loaded yet`, true)
			return
		}
		await computeTriggerables()
		try {
			let { css, js } = await getBundle()
			await AppService.createAppRaw({
				workspace: $workspaceStore!,
				formData: {
					app: {
						value: app,
						path: newEditedPath,
						summary: summary,
						policy,
						draft_only: true,
						custom_path: customPath
					},
					js,
					css
				}
			})
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: newEditedPath,
					typ: 'app',
					value: {
						value: app,
						path: newEditedPath,
						summary: summary,
						policy,
						custom_path: customPath
					}
				}
			})
			savedApp = {
				summary: summary,
				value: structuredClone(stateSnapshot(app)),
				path: newEditedPath,
				policy,
				draft_only: true,
				draft: {
					summary: summary,
					value: structuredClone(stateSnapshot(app)),
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
		if (!app) {
			sendUserToast(`App hasn't been loaded yet`, true)
			return
		}
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
			summary: summary,
			value: app,
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
			let path = appPath
			if (savedApp.draft_only) {
				await AppService.deleteApp({
					workspace: $workspaceStore!,
					path: path
				})
				let { css, js } = await getBundle()

				await AppService.createAppRaw({
					workspace: $workspaceStore!,
					formData: {
						app: {
							value: app!,
							summary: summary,
							policy,
							path: newEditedPath || path,
							draft_only: true,
							custom_path: customPath
						},
						js,
						css
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: savedApp.draft_only ? newEditedPath || path : path,
					typ: 'app',
					value: {
						value: app!,
						summary: summary,
						policy,
						path: newEditedPath || path
					}
				}
			})

			savedApp = {
				...(savedApp?.draft_only
					? {
							summary: summary,
							value: structuredClone(stateSnapshot(app)),
							path: savedApp.draft_only ? newEditedPath || path : path,
							policy,
							draft_only: true,
							custom_path: customPath
						}
					: savedApp),
				draft: {
					summary: summary,
					value: structuredClone(stateSnapshot(app)),
					path: newEditedPath || path,
					policy,
					custom_path: customPath
				}
			}

			sendUserToast('Draft saved')
			try {
				localStorage.removeItem(`rawapp-${path}`)
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
				path: appPath
			})
			onLatest = appVersion?.version === undefined || version === appVersion?.version
		} catch (e) {
			console.error('Error comparing versions', e)
			onLatest = true
		}
	}

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
				appExport?.open(app)
			}
		},
		{
			displayName: 'Publish to Hub',
			icon: Globe,
			action: () => {
				publishToHubDrawerOpen = true
			}
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
						summary: summary,
						value: app,
						path: newEditedPath || savedApp.draft?.path || savedApp.path,
						policy,
						custom_path: customPath
					}
				})
			},
			disabled: !savedApp
		}
	]

	const dispatch = createEventDispatcher()

	let customPath = $state(savedApp?.custom_path)
	let customPathError = $state('')

	let jobsDrawerOpen = $state(false)

	function getInitialAndModifiedValues(): SavedAndModifiedValue {
		return {
			savedValue: savedApp,
			modifiedValue: {
				summary: summary,
				value: app,
				path: newEditedPath || savedApp?.draft?.path || savedApp?.path,
				policy,
				custom_path: customPath
			}
		}
	}
	let app = $derived(files ? { runnables: runnables, files, data } : undefined)

	$effect(() => {
		saveDrawerOpen && compareVersions()
	})
</script>

<UnsavedConfirmationModal {diffDrawer} {getInitialAndModifiedValues} />

<DeployOverrideConfirmationModal
	{deployedBy}
	{confirmCallback}
	bind:open
	{diffDrawer}
	bind:deployedValue
	currentValue={{
		summary: summary,
		value: app,
		path: newEditedPath || savedApp?.draft?.path || savedApp?.path,
		policy,
		custom_path: customPath
	}}
/>

{#if appPath == ''}
	<Drawer bind:open={draftDrawerOpen} size="800px">
		<DrawerContent title="Initial draft save" on:close={() => closeDraftDrawer()}>
			{#snippet actions()}
				<div>
					<Button
						startIcon={{ icon: Save }}
						disabled={pathError != '' || app == undefined}
						on:click={() => saveInitialDraft()}
						unifiedSize="md"
						variant="accent"
					>
						Save initial draft
					</Button>
				</div>
			{/snippet}
			<AppEditorHeaderDeployInitialDraft
				bind:summary
				bind:appPath
				bind:pathError
				bind:newEditedPath
			/>
		</DrawerContent>
	</Drawer>
{/if}
<Drawer bind:open={saveDrawerOpen} size="800px">
	<DrawerContent title="Deploy" on:close={() => closeSaveDrawer()}>
		{#snippet actions()}
			<div class="flex flex-row gap-2">
				<Button
					variant="default"
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
								summary: summary,
								value: app,
								path: newEditedPath || savedApp.draft?.path || savedApp.path,
								policy,
								custom_path: customPath
							},
							button: {
								text: 'Looks good, deploy',
								onClick: () => {
									if (appPath == '') {
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
						if (appPath == '') {
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
			{appPath}
			{onLatest}
			{savedApp}
			bind:summary
			bind:customPath
			bind:deploymentMsg
			bind:customPathError
			bind:pathError
			bind:newEditedPath
		/>
	</DrawerContent>
</Drawer>

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory on:restore {appPath} />
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
	class="border-b flex flex-row justify-between py-1 gap-2 gap-y-2 px-2 items-center overflow-y-visible overflow-x-auto min-h-10 shrink-0"
>
	<div class="flex flex-row gap-2 items-center">
		<Summary bind:value={summary} />
		<div></div>
		<UndoRedo
			undoProps={{ disabled: !canUndo }}
			redoProps={{ disabled: !canRedo }}
			on:undo={() => onUndo?.()}
			on:redo={() => onRedo?.()}
		/>
	</div>

	<div class=" flex">
		{#if newPath || newEditedPath}
			<div class="flex justify-start w-full border rounded-md overflow-hidden">
				<div>
					<button
						onclick={async () => {
							saveDrawerOpen = true
							setTimeout(() => {
								document.getElementById('path')?.focus()
							}, 100)
						}}
					>
						<Badge
							color="gray"
							class="center-center !bg-surface-secondary !text-primary !h-[28px]  !w-[70px] rounded-none hover:!bg-surface-hover transition-all flex gap-1"
						>
							<Pen size={14} />Path
						</Badge>
					</button>
				</div>
				<input
					type="text"
					readonly
					value={defaultIfEmptyString(newEditedPath, newPath)}
					size={defaultIfEmptyString(newEditedPath, newPath)?.length || 50}
					class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full !h-[28px] !my-0 !py-0 !border-l-0 !rounded-l-none !border-0 !shadow-none"
					onfocus={({ currentTarget }) => {
						currentTarget.select()
					}}
				/>
			</div>
		{/if}
	</div>
	{#if $enterpriseLicense && appPath != ''}
		<Awareness />
	{/if}
	<div class="flex flex-row gap-2 justify-end items-center overflow-visible">
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

		<div class="hidden md:inline relative overflow-visible">
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
		<Button
			loading={loading.save}
			startIcon={{ icon: Save }}
			on:click={() => saveDraft()}
			unifiedSize="md"
			variant="default"
			disabled={!newApp && !savedApp}
			shortCut={{ key: 'S' }}
		>
			Draft
		</Button>
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
						}
					]
				: undefined}
		>
			Deploy
		</Button>
	</div>
</div>
