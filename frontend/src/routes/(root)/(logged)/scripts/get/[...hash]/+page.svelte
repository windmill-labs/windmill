<script lang="ts">
	import { page } from '$app/stores'
	import { JobService, ScriptService, type Script } from '$lib/gen'
	import {
		defaultIfEmptyString,
		emptyString,
		encodeState,
		canWrite,
		truncateHash
	} from '$lib/utils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { runFormStore, userStore, workspaceStore } from '$lib/stores'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import { onDestroy } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import {
		Tabs,
		Tab,
		TabContent,
		Badge,
		Alert,
		DrawerContent,
		Drawer
	} from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import { goto } from '$app/navigation'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'

	import { sendUserToast } from '$lib/toast'
	import Urlize from '$lib/components/Urlize.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'

	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import CliHelpBox from '$lib/components/CliHelpBox.svelte'
	import {
		Archive,
		ArchiveRestore,
		FolderOpen,
		GitFork,
		Globe2,
		History,
		Loader2,
		Pen,
		Server,
		Share,
		Table2,
		Trash
	} from 'lucide-svelte'
	import { SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB } from '$lib/consts'
	import { scriptToHubUrl } from '$lib/hub'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import ScriptVersionHistory from '$lib/components/ScriptVersionHistory.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { createAppFromScript } from '$lib/components/details/createAppFromScript'
	import { importStore } from '$lib/components/apps/store'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'

	let script: Script | undefined
	let topHash: string | undefined
	let can_write = false
	let deploymentInProgress = false
	let intervalId: NodeJS.Timer
	let shareModal: ShareModal
	let runForm: RunForm

	$: cliCommand = `wmill script run ${script?.path} -d '${JSON.stringify(args)}'`

	$: loading = !script
	$: if ($workspaceStore) {
		loadScript($page.params.hash)
	}
	$: webhooks = {
		async: {
			hash: `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/h/${script?.hash}`,
			path: `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/p/${script?.path}`
		},
		sync: {
			hash: `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run_wait_result/h/${script?.hash}`,
			path: `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run_wait_result/p/${script?.path}`,
			get_path: `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run_wait_result/p/${script?.path}`
		}
	}

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
				lock: r.lock?.split('\n')
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
				script.lock = status.lock
				script.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			}
		}
	}

	async function loadScript(hash: string): Promise<void> {
		try {
			script = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
		} catch {
			script = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: hash })
			hash = script.hash
		}
		can_write =
			script.workspace_id == $workspaceStore &&
			canWrite(script.path, script.extra_perms!, $userStore)
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
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})

	let isValid = true

	let runLoading = false
	async function runScript(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		try {
			runLoading = true
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
			let run = await JobService.runScriptByHash({
				workspace: $workspaceStore!,
				hash: script?.hash ?? '',
				requestBody: args,
				scheduledFor,
				invisibleToOwner
			})
			await goto('/run/' + run + '?workspace=' + $workspaceStore)
		} catch (err) {
			runLoading = false
			sendUserToast(`Could not create job: ${err.body}`, true)
		}
	}

	let args = undefined

	if ($runFormStore) {
		args = $runFormStore
		$runFormStore = undefined
	}

	let moveDrawer: MoveDrawer
	let deploymentDrawer: DeployWorkspaceDrawer

	function getMainButtons(script: Script | undefined, args: object | undefined, topHash?: string) {
		const buttons: any = []

		if (!topHash && script) {
			buttons.push({
				label: 'Fork',
				buttonProps: {
					href: `/scripts/add?template=${script.path}`,
					size: 'xs',
					color: 'light',
					startIcon: GitFork
				}
			})
		}

		if (!script || $userStore?.operator || !can_write) {
			return buttons
		}

		buttons.push({
			label: `View runs`,
			buttonProps: {
				href: `/runs/${script.path}`,
				size: 'xs',
				color: 'light',
				startIcon: History
			}
		})

		if (Array.isArray(script.parent_hashes) && script.parent_hashes.length > 0) {
			buttons.push({
				label: `History`,
				buttonProps: {
					onClick: () => {
						versionsDrawerOpen = !versionsDrawerOpen
					},

					size: 'xs',
					color: 'light',
					startIcon: History
				}
			})
		}

		if (!$userStore?.operator) {
			buttons.push({
				label: 'Build App',
				buttonProps: {
					onClick: async () => {
						const app = createAppFromScript(script.path, script.schema)
						$importStore = JSON.parse(JSON.stringify(app))
						await goto('/apps/add?nodraft=true')
					},

					size: 'xs',
					color: 'light',
					startIcon: Table2
				}
			})

			buttons.push({
				label: 'Edit',
				buttonProps: {
					href: `/scripts/edit/${script.path}?args=${encodeState(args)}${
						topHash ? `&hash=${script.hash}&topHash=` + topHash : ''
					}`,
					size: 'xs',
					startIcon: Pen,
					color: 'dark',
					variant: 'contained',
					disabled: !can_write
				}
			})
		}

		return buttons
	}
	$: mainButtons = getMainButtons(script, args, topHash)

	function getMenuItems(script: Script | undefined) {
		if (!script || $userStore?.operator) return []

		const menuItems: any = []

		menuItems.push({
			label: 'Move/Rename',
			Icon: FolderOpen,
			onclick: () => {
				moveDrawer.openDrawer(script?.path ?? '', script?.summary, 'script')
			}
		})

		menuItems.push({
			label: 'Share',
			Icon: Share,
			onclick: () => {
				shareModal.openDrawer(script?.path ?? '', 'script')
			}
		})

		menuItems.push({
			label: 'Deploy to staging/prod',
			Icon: Server,
			onclick: () => {
				deploymentDrawer.openDrawer(script?.path ?? '', 'script')
			}
		})

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
							script.lock ?? ''
						).toString(),
						'_blank'
					)
				}
			})
		}

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
				deleteScript(script.path)
			},
			color: 'red'
		})

		return menuItems
	}

	let versionsDrawerOpen = false

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
</script>

<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/scripts/get/' + e.detail + `?workspace=${$workspaceStore}`)
		loadScript($page.params.hash)
	}}
/>

<svelte:window on:keydown={onKeyDown} />

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ShareModal bind:this={shareModal} />

{#if script}
	<Drawer bind:open={versionsDrawerOpen} size="1200px">
		<DrawerContent title="Versions History" on:close={() => (versionsDrawerOpen = false)}>
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
		</DrawerContent>
	</Drawer>
	<DetailPageLayout isOperator={$userStore?.operator}>
		<svelte:fragment slot="header">
			<DetailPageHeader
				{mainButtons}
				menuItems={getMenuItems(script)}
				title={defaultIfEmptyString(script.summary, script.path)}
				bind:errorHandlerMuted={script.ws_error_handler_muted}
				errorHandlerKind="script"
				scriptOrFlowPath={script.path}
				tag={script.tag}
			>
				{#if script?.priority != undefined}
					<div class="hidden md:block">
						<Badge color="red" variant="outlined" size="xs">
							{`Priority: ${script.priority}`}
						</Badge>
					</div>
				{/if}
				{#if script?.concurrent_limit != undefined && script.concurrency_time_window_s != undefined}
					<div class="hidden md:block">
						<Badge color="gray" variant="outlined" size="xs">
							{`Concurrency limit: ${script.concurrent_limit} runs every ${script.concurrency_time_window_s}s`}
						</Badge>
					</div>
				{/if}
			</DetailPageHeader>
		</svelte:fragment>
		<svelte:fragment slot="form">
			<div class="p-8 w-full max-w-3xl mx-auto">
				<div class="flex flex-col gap-0.5 mb-8">
					{#if script.lock_error_logs || topHash || script.archived || script.deleted}
						<div class="flex flex-col gap-2 my-2">
							{#if script.lock_error_logs}
								<div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4" role="alert">
									<p class="font-bold">Error deploying this script</p>
									<p>
										This script has not been deployed successfully because of the following errors:
									</p>
									<pre class="w-full text-xs mt-2 whitespace-pre-wrap">{script.lock_error_logs}</pre
									>
								</div>
							{/if}
							{#if topHash}
								<div class="mt-2" />
								<Alert type="warning" title="Not HEAD">
									This hash is not HEAD (latest non-archived version at this path) :
									<a href="/scripts/get/{topHash}?workspace={$workspaceStore}"
										>Go to the HEAD of this path</a
									>
								</Alert>
							{/if}
							{#if script.archived && !topHash}
								<Alert type="error" title="Archived">This path was archived</Alert>
							{/if}
							{#if script.deleted}
								<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4" role="alert">
									<p class="font-bold">Deleted</p>
									<p>The content of this script was deleted (by an admin, no less)</p>
								</div>
							{/if}
						</div>
					{/if}

					{#if !emptyString(script.summary)}
						<span class="text-lg font-semibold">{script.path}</span>
					{/if}

					<div class="flex flex-row gap-x-2 flex-wrap items-center mt-2">
						<span class="text-sm text-secondary">
							Edited <TimeAgo date={script.created_at || ''} /> by {script.created_by || 'unknown'}
						</span>
						<Badge small color="dark-blue">
							{truncateHash(script?.hash ?? '')}
						</Badge>
						{#if script?.is_template}
							<Badge color="blue">Template</Badge>
						{/if}
						{#if script && script.kind !== 'script'}
							<Badge color="blue">
								{script?.kind}
							</Badge>
						{/if}
						{#if deploymentInProgress}
							<Badge color="yellow">
								<Loader2 size={12} class="inline animate-spin mr-1" />
								Deployment in progress
							</Badge>
						{/if}
						<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
					</div>

					{#if !emptyString(script.description)}
						<div class="border p-2">
							<Urlize text={defaultIfEmptyString(script.description, 'No description')} />
						</div>
					{/if}
				</div>

				<RunForm
					viewKeybinding
					loading={runLoading}
					autofocus
					detailed={false}
					bind:isValid
					runnable={script}
					runAction={runScript}
					bind:args
					schedulable={true}
					isFlow={false}
					bind:this={runForm}
				/>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="save_inputs">
			{#if args}
				<SavedInputs
					scriptPath={script?.path}
					scriptHash={topHash}
					{isValid}
					{args}
					on:selected_args={(e) => {
						const nargs = JSON.parse(JSON.stringify(e.detail))
						runForm?.setArgs(nargs)
						args = nargs
					}}
				/>
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="webhooks">
			<WebhooksPanel scopes={[`run:script/${script?.path}`]} {webhooks} {args} />
		</svelte:fragment>
		<svelte:fragment slot="schedule">
			<RunPageSchedules isFlow={false} path={script.path ?? ''} {can_write} />
		</svelte:fragment>
		<svelte:fragment slot="details">
			<div>
				<Skeleton {loading} layout={[[20]]} />

				<Tabs selected="code">
					<Tab value="code" size="xs">Code</Tab>
					<Tab value="dependencies" size="xs">Lockfile</Tab>
					<Tab value="arguments" size="xs">
						<span class="inline-flex items-center gap-1">
							Inputs
							<Tooltip>
								The jsonschema defines the constraints that the payload must respect to be
								compatible with the input parameters of this script. The UI form is generated
								automatically from the script jsonschema. See
								<a href="https://json-schema.org/" class="text-blue-500">
									jsonschema documentation
								</a>
							</Tooltip>
						</span>
					</Tab>
					<svelte:fragment slot="content">
						<TabContent value="code">
							<div class="p-2 w-full overflow-auto">
								<HighlightCode
									language={script.language}
									code={script.content}
									class="whitespace-pre"
								/>
							</div>
						</TabContent>
						<TabContent value="dependencies">
							<div>
								{#if script?.lock}
									<pre class="bg-surface-secondary text-sm p-2 h-full overflow-auto w-full"
										>{script.lock}</pre
									>
								{:else}
									<p class="bg-surface-secondary text-sm p-2">
										There is no lock file for this script
									</p>
								{/if}
							</div>
						</TabContent>
						<TabContent value="arguments">
							<div class="p-2">
								<SchemaViewer schema={script.schema} />
							</div>
						</TabContent>
					</svelte:fragment>
				</Tabs>
			</div>

			{#if script.envs && script.envs.length > 0}
				<h3>Static Env Variables</h3>
				<ul>
					{#each script.envs as e}
						<li>{e}</li>
					{/each}
				</ul>
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="cli">
			<div class="p-2 flex flex-col gap-4">
				<ClipboardPanel content={cliCommand} />
				<CliHelpBox />
			</div>
		</svelte:fragment>
	</DetailPageLayout>
{/if}
