<script lang="ts">
	import { base } from '$lib/base'
	import {
		JobService,
		ScriptService,
		WorkspaceService,
		type Script,
		type TriggersCount,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import {
		defaultIfEmptyString,
		emptyString,
		canWrite,
		truncateHash,
		copyToClipboard,
		urlParamsToObject
	} from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { enterpriseLicense, hubBaseUrlStore, userStore, workspaceStore } from '$lib/stores'
	import { isDeployable, ALL_DEPLOYABLE } from '$lib/utils_deployable'
	import AIFormAssistant from '$lib/components/copilot/AIFormAssistant.svelte'

	import { onDestroy, setContext, tick, untrack } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import {
		Tabs,
		Tab,
		TabContent,
		Badge,
		Alert,
		DrawerContent,
		Drawer,
		Button
	} from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import { goto } from '$lib/navigation'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'

	import { sendUserToast } from '$lib/toast'
	import NoDirectDeployAlert from '$lib/components/NoDirectDeployAlert.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'

	import SavedInputsV2 from '$lib/components/SavedInputsV2.svelte'
	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import {
		Activity,
		Archive,
		ArchiveRestore,
		Eye,
		FolderOpen,
		GitFork,
		Globe2,
		History,
		Loader2,
		Pen,
		ChevronUpSquare,
		Share,
		Trash,
		Play,
		ClipboardCopy,
		LayoutDashboard
	} from 'lucide-svelte'
	import { SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB } from '$lib/consts'
	import { scriptToHubUrl } from '$lib/hub'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ScriptVersionHistory from '$lib/components/ScriptVersionHistory.svelte'
	import { createAppFromScript } from '$lib/components/details/createAppFromScript'
	import { importStore } from '$lib/components/apps/store'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import PersistentScriptDrawer from '$lib/components/PersistentScriptDrawer.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import Star from '$lib/components/Star.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { writable } from 'svelte/store'
	import Toggle from '$lib/components/Toggle.svelte'
	import InputSelectedBadge from '$lib/components/schema/InputSelectedBadge.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { slide } from 'svelte/transition'
	import TriggersBadge from '$lib/components/graph/renderers/triggers/TriggersBadge.svelte'
	import TriggersEditor from '$lib/components/triggers/TriggersEditor.svelte'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'
	import { page } from '$app/state'

	let script: Script | undefined = $state()
	let topHash: string | undefined = $state()
	let can_write = $state(false)
	let isHubScript = $state(false)
	let deploymentInProgress = $state(false)
	let deploymentJobId: string | undefined = $state(undefined)
	let intervalId: number
	let shareModal: ShareModal | undefined = $state()
	let runForm: RunForm | undefined = $state()

	let scheduledForStr: string | undefined = $state(undefined)
	let invisible_to_owner: boolean | undefined = $state(undefined)
	let overrideTag: string | undefined = $state(undefined)
	let inputSelected: 'saved' | 'history' | undefined = $state(undefined)
	let jsonView = $state(false)

	let previousHash: string | undefined = $state(undefined)

	const triggersCount = writable<TriggersCount | undefined>(undefined)

	// Add triggers context store
	const triggersState = $state(
		new Triggers([
			{ type: 'webhook', path: '', isDraft: false },
			{ type: 'default_email', path: '', isDraft: false },
			{ type: 'cli', path: '', isDraft: false }
		])
	)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState
	})

	async function deleteScript(hash: string): Promise<void> {
		try {
			await ScriptService.deleteScriptByHash({ workspace: $workspaceStore!, hash })
			loadScript(hash)
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not delete this script ${err.body}`, true)
		}
	}

	async function archiveScript(hash: string): Promise<void> {
		await ScriptService.archiveScriptByHash({ workspace: $workspaceStore!, hash })
		loadScript(hash)
	}

	async function unarchiveScript(hash: string): Promise<void> {
		const r = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
		const ns = await ScriptService.createScript({
			workspace: $workspaceStore!,
			requestBody: {
				...r,
				parent_hash: hash,
				lock: r.lock
			}
		})
		sendUserToast(`Unarchived script`)
		loadScript(ns)
		goto(`/scripts/get/${ns}`)
	}

	async function syncer(): Promise<void> {
		if (script?.hash) {
			const status = await ScriptService.getScriptDeploymentStatus({
				workspace: $workspaceStore!,
				hash: script?.hash!
			})
			if (status.lock != undefined || status.lock_error_logs != undefined) {
				deploymentInProgress = false
				deploymentJobId = undefined
				script.lock = status.lock
				script.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			} else if (status.job_id) {
				deploymentJobId = status.job_id
			}
		}
	}

	async function loadTriggers(path: string): Promise<void> {
		await triggersState.fetchTriggers(
			triggersCount,
			$workspaceStore,
			path,
			false,
			undefined,
			$userStore
		)
	}

	async function loadScript(hash: string): Promise<void> {
		// Check if this is a hub script path
		if (hash.startsWith('hub/')) {
			isHubScript = true
			try {
				const hubScript = await ScriptService.getHubScriptByPath({ path: hash })
				// Create a partial Script object from hub script data
				script = {
					hash: '',
					path: hash,
					summary: hubScript.summary ?? '',
					description: '',
					content: hubScript.content,
					created_by: '',
					created_at: '',
					archived: false,
					deleted: false,
					is_template: false,
					extra_perms: {},
					lock: hubScript.lockfile,
					language: hubScript.language as Script['language'],
					kind: 'script',
					starred: false,
					schema: hubScript.schema as Script['schema'],
					no_main_func: false,
					has_preprocessor: false
				}
				can_write = false
				return
			} catch (e) {
				sendUserToast('Could not load hub script: ' + e.body, true)
				return
			}
		}

		isHubScript = false
		try {
			script = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash,
				withStarredInfo: true,
				authed: true
			})
		} catch {
			try {
				script = await ScriptService.getScriptByPath({
					workspace: $workspaceStore!,
					path: hash,
					withStarredInfo: true
				})
				hash = script.hash
			} catch (e) {
				sendUserToast('Could not load script: ' + e.body, true)
				return
			}
		}
		can_write =
			script.workspace_id == $workspaceStore &&
			canWrite(script.path, script.extra_perms!, $userStore)
		loadTriggers(script.path)

		if (script.path && script.archived) {
			const script_by_path = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: script.path
			}).catch((_) => console.error('this script has no non-archived version'))
			if (script_by_path?.hash != script.hash) {
				topHash = script_by_path?.hash
			}
		} else {
			topHash = undefined
		}
		intervalId && clearInterval(intervalId)
		deploymentInProgress = script.lock == undefined && script.lock_error_logs == undefined
		if (deploymentInProgress) {
			intervalId = setInterval(syncer, 500)
		}
		if (!script.path.startsWith(`u/${$userStore?.username}`) && script.path.split('/').length > 2) {
			invisible_to_owner = script.visible_to_runner_only
		}
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})

	let isValid = $state(true)

	let runLoading = $state(false)
	async function runScript(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner: boolean | undefined,
		overrideTag: string | undefined
	) {
		try {
			runLoading = true
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
			let run: string
			if (isHubScript) {
				run = await JobService.runScriptByPath({
					workspace: $workspaceStore!,
					path: script?.path ?? '',
					requestBody: args,
					scheduledFor,
					invisibleToOwner,
					tag: overrideTag,
					skipPreprocessor: true
				})
			} else {
				run = await JobService.runScriptByHash({
					workspace: $workspaceStore!,
					hash: script?.hash ?? '',
					requestBody: args,
					scheduledFor,
					invisibleToOwner,
					tag: overrideTag,
					skipPreprocessor: true
				})
			}
			await goto('/run/' + run + '?workspace=' + $workspaceStore)
		} catch (err) {
			runLoading = false
			sendUserToast(`Could not create job: ${err.body}`, true)
		}
	}

	let args: Record<string, any> | undefined = $state(undefined)
	let hash = window.location.hash
	if (hash.length > 1) {
		try {
			let searchParams = new URLSearchParams(hash.slice(1))
			let params = [...Object.entries(urlParamsToObject(searchParams))].map(([k, v]) => [
				k,
				JSON.parse(v)
			])
			args = Object.fromEntries(params)
		} catch (e) {
			console.error('Was not able to transform hash as args', e)
		}
	}

	let moveDrawer: MoveDrawer | undefined = $state()
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state()
	let persistentScriptDrawer: PersistentScriptDrawer | undefined = $state()
	let showEditButtons = $state(false)

	function getMainButtons(
		script: Script | undefined,
		args: object | undefined,
		topHash?: string,
		can_write?: boolean
	) {
		const buttons: any = []

		if (!topHash && script && !$userStore?.operator && !script.codebase) {
			buttons.push({
				label: 'Fork',
				buttonProps: {
					href: `${base}/scripts/add?template=${script.path}`,
					unifiedSize: 'md',
					variant: 'subtle',
					disabled: !showEditButtons,
					startIcon: GitFork
				}
			})
		}

		if (!script) {
			return buttons
		}

		buttons.push({
			label: `Runs`,
			buttonProps: {
				href: `${base}/runs/${script.path}`,
				unifiedSize: 'md',
				variant: 'subtle',
				startIcon: Play
			}
		})

		if (!script || $userStore?.operator || !can_write) {
			return buttons
		}

		if (Array.isArray(script.parent_hashes) && script.parent_hashes.length > 0) {
			buttons.push({
				label: `History`,
				buttonProps: {
					onClick: () => {
						versionsDrawerOpen = !versionsDrawerOpen
					},

					unifiedSize: 'md',
					variant: 'subtle',
					startIcon: History
				}
			})
		}

		if (!$userStore?.operator) {
			buttons.push({
				label: 'Build app',
				buttonProps: {
					onClick: async () => {
						const app = createAppFromScript(script.path, script.schema)
						$importStore = JSON.parse(JSON.stringify(app))
						await goto('/apps/add?nodraft=true')
					},
					disabled: !showEditButtons,
					unifiedSize: 'md',
					variant: 'subtle',
					startIcon: LayoutDashboard
				}
			})

			if (script?.restart_unless_cancelled ?? false) {
				buttons.push({
					label: 'Current runs',
					buttonProps: {
						onClick: () => {
							persistentScriptDrawer?.open?.(script)
						},
						unifiedSize: 'md',
						startIcon: Activity,
						variant: 'accent'
					}
				})
			}

			if (!script.codebase) {
				buttons.push({
					label: 'Edit',
					buttonProps: {
						href: `${base}/scripts/edit/${script.path}?${
							topHash ? `&hash=${script.hash}&topHash=` + topHash : ''
						}`,
						unifiedSize: 'md',
						startIcon: Pen,
						variant: 'accent',
						disabled: !can_write || !showEditButtons
					}
				})
			}
		}

		return buttons
	}

	let deployUiSettings: WorkspaceDeployUISettings | undefined = $state(undefined)

	async function getDeployUiSettings() {
		if (!$enterpriseLicense) {
			deployUiSettings = ALL_DEPLOYABLE
			return
		}
		let settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		deployUiSettings = settings.deploy_ui ?? ALL_DEPLOYABLE
	}
	getDeployUiSettings()

	function getMenuItems(
		script: Script | undefined,
		deployUiSettings: WorkspaceDeployUISettings | undefined
	) {
		if (!script || $userStore?.operator) return []

		const menuItems: any = []

		if (showEditButtons) {
			menuItems.push({
				label: 'Move/Rename',
				Icon: FolderOpen,
				onclick: () => {
					moveDrawer?.openDrawer(script?.path ?? '', script?.summary, 'script')
				}
			})
		}

		menuItems.push({
			label: 'Audit logs',
			Icon: Eye,
			onclick: () => {
				goto(`/audit_logs?resource=${script?.path}`)
			}
		})

		menuItems.push({
			label: 'Share',
			Icon: Share,
			onclick: () => {
				shareModal?.openDrawer(script?.path ?? '', 'script')
			}
		})

		if (isDeployable('script', script?.path ?? '', deployUiSettings)) {
			menuItems.push({
				label: 'Deploy to staging/prod',
				Icon: ChevronUpSquare,
				onclick: () => {
					deploymentDrawer?.openDrawer(script?.path ?? '', 'script')
				}
			})
		}

		if (SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB) {
			menuItems.push({
				label: 'Publish to Hub',
				Icon: Globe2,
				onclick: () => {
					if (!script) return

					window.open(
						scriptToHubUrl(
							script.content,
							script.summary,
							script.description ?? '',
							script.kind,
							script.language,
							script.schema,
							script.lock ?? '',
							$hubBaseUrlStore
						).toString(),
						'_blank'
					)
				}
			})
		}

		if (showEditButtons) {
			if (script.archived) {
				menuItems.push({
					label: 'Unarchive',
					Icon: ArchiveRestore,
					onclick: async () => {
						unarchiveScript(script.hash)
					},
					color: 'red'
				})
			} else {
				menuItems.push({
					label: 'Archive',
					Icon: Archive,
					onclick: async () => {
						archiveScript(script.hash)
					},
					color: 'red'
				})
			}

			menuItems.push({
				label: 'Delete',
				Icon: Trash,
				onclick: async () => {
					deleteScript(script.hash)
				},
				color: 'red'
			})
		}

		return menuItems
	}

	let versionsDrawerOpen = $state(false)

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey || event.metaKey) {
					if (isValid) {
						event.preventDefault()
						runForm?.run()
					} else {
						sendUserToast('Please fix errors before running', true)
					}
				}
				break
		}
	}

	let rightPaneSelected = $state('saved_inputs')

	let savedInputsV2: SavedInputsV2 | undefined = $state(undefined)
	$effect(() => {
		const cliTrigger = triggersState.triggers.find((t) => t.type === 'cli')
		if (cliTrigger) {
			cliTrigger.extra = {
				cliCommand: `wmill script run ${script?.path} -d '${JSON.stringify(args)}'`
			}
		}
	})
	let loading = $derived(!script)
	$effect(() => {
		if ($workspaceStore) {
			if (previousHash != page.params.hash) {
				previousHash = page.params.hash
				untrack(() => loadScript(page.params.hash ?? ''))
			}
		}
	})
	let mainButtons = $derived(getMainButtons(script, args, topHash, can_write))
</script>

<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/scripts/get/' + e.detail + `?workspace=${$workspaceStore}`)
		loadScript(page.params.hash ?? '')
	}}
/>

<svelte:window onkeydown={onKeyDown} />

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<PersistentScriptDrawer bind:this={persistentScriptDrawer} />
<ShareModal bind:this={shareModal} />

<Drawer bind:open={versionsDrawerOpen} size="1200px">
	<DrawerContent title="Versions History" on:close={() => (versionsDrawerOpen = false)} noPadding>
		{#if script}
			<ScriptVersionHistory
				scriptPath={script.path}
				openDetails
				on:openDetails={(e) => {
					if (script) {
						goto(`/scripts/get/${e.detail.version}?workspace=${$workspaceStore}`)
					}
					versionsDrawerOpen = false
				}}
			/>
		{/if}
	</DrawerContent>
</Drawer>
{#key script?.hash}
	<DetailPageLayout bind:selected={rightPaneSelected} isOperator={$userStore?.operator}>
		{#snippet header()}
			<DetailPageHeader
				{mainButtons}
				menuItems={getMenuItems(script, deployUiSettings)}
				bind:errorHandlerMuted={
					() => script?.ws_error_handler_muted ?? false,
					(v) => {
						if (script !== undefined) script.ws_error_handler_muted = v
					}
				}
				errorHandlerKind="script"
				scriptOrFlowPath={script?.path ?? ''}
				tag={script?.tag ?? ''}
				on:seeTriggers={() => {
					rightPaneSelected = 'triggers'
				}}
				summary={script?.summary}
				path={script?.path}
				onSaved={can_write
					? async (newPath) => {
							if (newPath !== script?.path) {
								await goto(`/scripts/get/${newPath}?workspace=${$workspaceStore}`)
							} else {
								loadScript(page.params.hash ?? '')
							}
						}
					: undefined}
			>
				{#snippet trigger_badges()}
					<TriggersBadge
						showOnlyWithCount={true}
						showDraft={false}
						path={script?.path ?? ''}
						newItem={false}
						isFlow={false}
						selected={rightPaneSelected === 'triggers'}
						onSelect={async (triggerIndex: number) => {
							if (rightPaneSelected !== 'triggers') {
								rightPaneSelected = 'triggers'
							}
							await tick()
							triggersState.selectedTriggerIndex = triggerIndex
						}}
					/>
				{/snippet}
				{#if $workspaceStore && script}
					<Star kind="script" path={script.path} summary={script.summary} />
				{/if}
				{#if script?.codebase}
					<Badge
						>bundle<Tooltip
							>This script is deployed as a bundle and can only be deployed from the CLI for now</Tooltip
						></Badge
					>
				{/if}
				{#if script?.priority != undefined}
					<div class="hidden md:block">
						<Badge color="blue" variant="outlined" size="xs">
							{`Priority: ${script.priority}`}
						</Badge>
					</div>
				{/if}
				{#if script?.restart_unless_cancelled ?? false}
					<button onclick={() => persistentScriptDrawer?.open?.(script)}>
						<div class="hidden md:block">
							<Badge color="red" variant="outlined" size="xs">Persistent</Badge>
						</div>
					</button>
				{/if}
				{#if script?.concurrent_limit != undefined && script.concurrency_time_window_s != undefined}
					<div class="hidden md:block">
						<Badge color="gray" variant="outlined" size="xs">
							{`Concurrency limit: ${script.concurrent_limit} runs every ${script.concurrency_time_window_s}s`}
						</Badge>
					</div>
				{/if}
			</DetailPageHeader>
		{/snippet}
		{#snippet form()}
			<div class="px-3">
				<NoDirectDeployAlert onUpdateCanEditStatus={(v) => (showEditButtons = v)} />
			</div>
			{#if script}
				<div class="p-8 w-full max-w-3xl mx-auto md:min-h-[300px] flex flex-col md:justify-center">
					<div class="flex flex-col gap-0.5 mb-1">
						{#if script.lock_error_logs || topHash || script.archived || script.deleted}
							<div class="flex flex-col gap-2 my-2">
								{#if script.lock_error_logs}
									<Alert type="error" title="Deployment failed">
										<p>
											This script has not been deployed successfully because of the following
											errors:
										</p>
										<LogViewer content={script.lock_error_logs} isLoading={false} tag={undefined} />
									</Alert>
								{/if}
								{#if topHash}
									<div class="mt-2"></div>
									<Alert type="warning" title="Not HEAD">
										This hash is not HEAD (latest non-archived version at this path) :
										<a href="{base}/scripts/get/{topHash}?workspace={$workspaceStore}"
											>Go to the HEAD of this path</a
										>
									</Alert>
								{/if}
								{#if script.archived && !topHash}
									<Alert type="error" title="Archived">This path was archived</Alert>
								{/if}
								{#if script.deleted}
									<Alert type="error" title="Deleted">
										<p>The content of this script was deleted (by an admin, no less)</p>
									</Alert>
								{/if}
							</div>
						{/if}

						{#if !emptyString(script.description)}
							<div class="p-4 rounded-md bg-surface-secondary">
								<GfmMarkdown
									md={defaultIfEmptyString(script?.description, 'No description')}
									noPadding
								/>
							</div>
							<div class="h-4"></div>
						{/if}
					</div>

					{#if deploymentInProgress}
						<div class="pb-4" transition:slide={{ duration: 150 }}>
							<Badge color="yellow">
								<Loader2 size={12} class="inline animate-spin mr-1" />
								Deployment in progress
								{#if deploymentJobId}
									<a
										href="/run/{deploymentJobId}?workspace={$workspaceStore}"
										class="underline"
										target="_blank">view job</a
									>
								{/if}
							</Badge>
						</div>
					{/if}

					<div class="flex flex-col align-left">
						{#if (script.schema && Object.keys(script.schema.properties ?? {}).length > 0) || inputSelected}
							{@const hasSchema =
								script.schema && Object.keys(script.schema.properties ?? {}).length > 0}
							<div
								class="flex flex-row justify-between min-h-12"
								transition:slide={{ duration: 150 }}
							>
								<InputSelectedBadge
									onReject={() => {
										savedInputsV2?.resetSelected()
									}}
									{inputSelected}
								/>
								{#if hasSchema}
									<Toggle
										bind:checked={jsonView}
										size="xs"
										options={{
											right: 'JSON',
											rightTooltip: 'Fill args from JSON'
										}}
										lightMode
										on:change={(e) => {
											runForm?.setCode(JSON.stringify(args ?? {}, null, '\t'))
										}}
									/>
								{/if}
							</div>
						{/if}

						{#if script?.schema?.prompt_for_ai !== undefined}
							<AIFormAssistant
								instructions={script.schema?.prompt_for_ai as string}
								onEditInstructions={() => {
									goto(`/scripts/edit/${script?.path}?metadata_open=true`)
								}}
								runnableType="script"
							/>
						{/if}

						<RunForm
							bind:scheduledForStr
							bind:invisible_to_owner
							bind:overrideTag
							viewKeybinding
							loading={runLoading}
							autofocus
							detailed={false}
							bind:isValid
							runnable={script}
							runAction={runScript}
							bind:args
							schedulable={true}
							bind:this={runForm}
							{jsonView}
						/>
					</div>

					<div class="pt-4 flex flex-row gap-1 w-full justify-end items-center">
						{#if !isHubScript}
							<span class="text-2xs text-secondary">
								Edited <TimeAgo date={script.created_at || ''} /> by {script.created_by ||
									'unknown'}
							</span>
						{/if}
						<div class="flex flex-row gap-x-2 flex-wrap items-center">
							{#if !isHubScript}
								<Badge small color="gray">
									{truncateHash(script?.hash ?? '')}
								</Badge>
							{/if}
							{#if script?.is_template}
								<Badge color="blue">Template</Badge>
							{/if}
							{#if script && script.kind !== 'script'}
								<Badge color="blue">
									{script?.kind}
								</Badge>
							{/if}

							<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
						</div>
					</div>
				</div>
			{/if}
		{/snippet}
		{#snippet save_inputs()}
			{#if args}
				<SavedInputsV2
					schema={script?.schema}
					bind:this={savedInputsV2}
					scriptPath={script?.path}
					scriptHash={topHash}
					{isValid}
					{jsonView}
					{args}
					bind:inputSelected
					on:selected_args={(e) => {
						const nargs = JSON.parse(JSON.stringify(e.detail))
						args = nargs
						if (jsonView) {
							runForm?.setCode(JSON.stringify(args ?? {}, null, '\t'))
						}
					}}
				/>
			{/if}
		{/snippet}
		{#snippet triggers()}
			{#if script}
				<TriggersEditor
					{args}
					runnableVersion={script.hash}
					initialPath={script.path}
					currentPath={script.path}
					noEditor={true}
					newItem={false}
					isFlow={false}
					schema={script.schema}
					isDeployed={true}
					noCapture={true}
					isEditor={false}
				/>
			{/if}
		{/snippet}
		{#snippet scriptRender()}
			<div class="h-full">
				<Skeleton {loading} layout={[[20]]} />

				<Tabs selected="code">
					<Tab value="code" label="Code" />
					<Tab value="dependencies" label="Lockfile" />
					<Tab value="schema" label="Schema" />
					{#snippet content()}
						{#if script}
							<TabContent value="code">
								<div class="p-2 w-full">
									<HighlightCode
										language={script.language}
										code={script.content}
										className="whitespace-pre-wrap"
									/>
								</div>
							</TabContent>
							<TabContent value="dependencies">
								<div>
									{#if script?.lock}
										<div class="relative overflow-x-auto w-full">
											<Button
												wrapperClasses="absolute top-2 right-2 z-20"
												on:click={() => copyToClipboard(script?.lock)}
												color="light"
												size="xs2"
												startIcon={{
													icon: ClipboardCopy
												}}
												iconOnly
											/>
											<pre class="bg-surface-secondary text-sm p-2 h-full overflow-auto w-full"
												>{script.lock}</pre
											>
										</div>
									{:else}
										<p class="bg-surface-secondary text-sm p-2">
											There is no lock file for this script
										</p>
									{/if}
								</div>
							</TabContent>
							<TabContent value="schema">
								<div class="p-1 relative h-full">
									<button
										onclick={() => copyToClipboard(JSON.stringify(script?.schema, null, 4))}
										class="absolute top-2 right-2"
									>
										<ClipboardCopy size={14} />
									</button>
									<Highlight language={json} code={JSON.stringify(script?.schema, null, 4)} />
								</div>
							</TabContent>
						{/if}
					{/snippet}
				</Tabs>
			</div>

			{#if script?.envs && script.envs.length > 0}
				<h3>Static Env Variables</h3>
				<ul>
					{#each script?.envs as e}
						<li>{e}</li>
					{/each}
				</ul>
			{/if}
		{/snippet}
	</DetailPageLayout>
{/key}
