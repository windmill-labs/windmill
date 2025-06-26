<script lang="ts">
	import { clone } from '$lib/utils'
	import { Alert, Badge, Drawer, DrawerContent } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Path from '$lib/components/Path.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { AppService, DraftService, type Policy } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import {
		Bug,
		DiffIcon,
		FileJson,
		FileUp,
		History,
		Loader2,
		MoreVertical,
		Pen,
		Save
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
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from '../apps/editor/DeploymentHistory.svelte'
	import Awareness from '$lib/components/Awareness.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'

	import Summary from '$lib/components/Summary.svelte'
	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { base } from '$lib/base'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import type { HiddenRunnable } from '../apps/types'
	import type { Writable } from 'svelte/store'
	import AppJobsDrawer from '../apps/editor/AppJobsDrawer.svelte'
	import type { Runnable } from '../apps/inputType'
	import { collectStaticFields, hash, type TriggerableV2 } from '../apps/editor/commonAppUtils'
	import type { SavedAndModifiedValue } from '../common/confirmationModal/unsavedTypes'
	import DropdownV2 from '../DropdownV2.svelte'

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
	// }

	export let summary: string
	export let policy: Policy
	export let diffDrawer: DiffDrawer | undefined = undefined
	export let savedApp:
		| {
				value: any
				draft?: any
				path: string
				summary: string
				policy: any
				draft_only?: boolean
				custom_path?: string
		  }
		| undefined = undefined
	export let version: number | undefined = undefined

	export let newApp: boolean
	export let newPath: string = ''
	export let appPath: string
	export let runnables: Writable<Record<string, HiddenRunnable>>
	export let files: Record<string, string> | undefined
	export let jobs: string[]
	export let jobsById: Record<string, any>
	export let getBundle: () => Promise<{
		js: string
		css: string
	}>

	let newEditedPath = ''

	$: app = files ? { runnables: $runnables, files } : undefined

	let deployedValue: Value | undefined = undefined // Value to diff against
	let deployedBy: string | undefined = undefined // Author
	let confirmCallback: () => void = () => {} // What happens when user clicks `override` in warning
	let open: boolean = false // Is confirmation modal open

	// const { app, summary, appPath, jobs, jobsById, staticExporter } = getContext('AppViewerContext')

	const loading = {
		publish: false,
		save: false,
		saveDraft: false
	}

	let pathError: string | undefined = undefined
	let appExport: AppExportButton

	let draftDrawerOpen = false
	let saveDrawerOpen = false
	let historyBrowserDrawerOpen = false
	let deploymentMsg: string | undefined = undefined

	function closeSaveDrawer() {
		saveDrawerOpen = false
	}

	function closeDraftDrawer() {
		draftDrawerOpen = false
	}

	async function computeTriggerables() {
		policy.execution_mode = 'publisher'
		policy.on_behalf_of_email = $userStore?.email
		policy.on_behalf_of = $userStore?.username.includes('@')
			? $userStore?.username
			: `u/${$userStore?.username}`
		policy.triggerables_v2 = Object.fromEntries(
			(await Promise.all(
				Object.values($runnables).map(async (runnable) => {
					return await processRunnable(runnable.name, runnable, runnable.fields)
				})
			)) as [string, TriggerableV2][]
		)
		return policy
	}

	async function processRunnable(
		id: string,
		runnable: Runnable,
		fields: Record<string, any>
	): Promise<[string, TriggerableV2] | undefined> {
		const staticInputs = collectStaticFields(fields)
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
					one_of_inputs: {},
					allow_user_resources: allowUserResources
				}
			]
		} else if (runnable?.type == 'runnableByPath') {
			let prefix = runnable.runType !== 'hubscript' ? runnable.runType : 'script'
			return [
				`${id}:${prefix}/${runnable.path}`,
				{
					static_inputs: staticInputs,
					one_of_inputs: {},
					allow_user_resources: allowUserResources
				}
			]
		}
	}

	async function createApp(path: string) {
		if (!app) {
			sendUserToast(`App hasn't been loaded yet`, true)
			return
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
				value: clone(app),
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
			value: clone(app),
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

	let secretUrl: string | undefined = undefined

	$: appPath && appPath != '' && secretUrl == undefined && getSecretUrl()

	async function getSecretUrl() {
		secretUrl = await AppService.getPublicSecretOfApp({
			workspace: $workspaceStore!,
			path: appPath
		})
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
				value: clone(app),
				path: newEditedPath,
				policy,
				draft_only: true,
				draft: {
					summary: summary,
					value: clone(app),
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
							value: clone(app),
							path: savedApp.draft_only ? newEditedPath || path : path,
							policy,
							draft_only: true,
							custom_path: customPath
						}
					: savedApp),
				draft: {
					summary: summary,
					value: clone(app),
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

	let onLatest = true
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

	$: saveDrawerOpen && compareVersions()

	let dirtyPath = false
	let path: Path | undefined = undefined

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
				appExport.open(app)
			}
		},
		{
			displayName: 'Hub compatible JSON',
			icon: FileUp,
			action: () => {
				sendUserToast('todo')
				// appExport.open(toStatic(app, $staticExporter, summary).app)
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

	let customPath = savedApp?.custom_path
	let dirtyCustomPath = false
	let customPathError = ''
	$: fullCustomUrl = `${window.location.origin}${base}/a/${
		isCloudHosted() ? $workspaceStore + '/' : ''
	}${customPath}`
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
	$: customPath !== undefined && validateCustomPath(customPath)

	let jobsDrawerOpen = false

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
</script>

<UnsavedConfirmationModal {diffDrawer} {getInitialAndModifiedValues} />

<DeployOverrideConfirmationModal
	bind:deployedBy
	bind:confirmCallback
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
			<Alert bgClass="mb-4" title="Require path" type="info">
				Choose a path to save the initial draft of the app.
			</Alert>
			<h3>Summary</h3>
			<div class="w-full pt-2">
				<!-- svelte-ignore a11y-autofocus -->
				<input
					autofocus
					type="text"
					placeholder="App summary"
					class="text-sm w-full font-semibold"
					on:keydown|stopPropagation
					bind:value={summary}
					on:keyup={() => {
						if (appPath == '' && summary?.length > 0 && !dirtyPath) {
							path?.setName(
								summary
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
						disabled={pathError != '' || app == undefined}
						on:click={() => saveInitialDraft()}
					>
						Save initial draft
					</Button>
				</div>
			{/snippet}
		</DrawerContent>
	</Drawer>
{/if}
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
			<!-- svelte-ignore a11y-autofocus -->
			<input
				autofocus
				type="text"
				placeholder="App summary"
				class="text-sm w-full"
				bind:value={summary}
				on:keydown|stopPropagation
				on:keyup={() => {
					if (appPath == '' && summary?.length > 0 && !dirtyPath) {
						path?.setName(
							summary
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
			<!-- svelte-ignore a11y-autofocus -->
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
		<div class="py-2"></div>
		{#if appPath == ''}
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
							on:input={() => {
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

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => (historyBrowserDrawerOpen = false)}>
		<DeploymentHistory on:restore {appPath} />
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
	class="border-b flex flex-row justify-between py-1 gap-2 gap-y-2 px-2 items-center overflow-y-visible overflow-x-auto min-h-10"
>
	<div class="flex flex-row gap-2 items-center">
		<Summary bind:value={summary} />
	</div>

	<div class=" flex">
		{#if newPath || newEditedPath}
			<div class="flex justify-start w-full border rounded-md overflow-hidden">
				<div>
					<button
						on:click={async () => {
							saveDrawerOpen = true
							setTimeout(() => {
								document.getElementById('path')?.focus()
							}, 100)
						}}
					>
						<Badge
							color="gray"
							class="center-center !bg-surface-secondary !text-tertiary !h-[28px]  !w-[70px] rounded-none hover:!bg-surface-hover transition-all flex gap-1"
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
					on:focus={({ currentTarget }) => {
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
			<svelte:fragment slot="buttonReplacement">
				<Button nonCaptureEvent size="xs" color="light">
					<div class="flex flex-row items-center">
						<MoreVertical size={14} />
					</div>
				</Button>
			</svelte:fragment>
		</DropdownV2>

		<div class="hidden md:inline relative overflow-visible">
			<Button
				on:click={() => {
					jobsDrawerOpen = true
				}}
				color="light"
				size="xs"
				variant="border"
				btnClasses="relative"
			>
				<div class="flex flex-row gap-1 items-center">
					<Bug size={14} />
					<div>Debug runs</div>

					<div class="text-2xs text-tertiary"
						>({jobs?.length > 99 ? '99+' : (jobs?.length ?? 0)})</div
					>
				</div>
			</Button>
		</div>
		<AppExportButton bind:this={appExport} />
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
