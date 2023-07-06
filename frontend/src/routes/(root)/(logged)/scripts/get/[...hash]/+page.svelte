<script lang="ts">
	import { page } from '$app/stores'
	import { JobService, ScriptService, type Script } from '$lib/gen'
	import { defaultIfEmptyString, emptyString, encodeState, canWrite } from '$lib/utils'
	import { faEdit, faCalendar, faCodeFork, faHistory } from '@fortawesome/free-solid-svg-icons'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import { onDestroy } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Tabs, Tab, TabContent, Button } from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import { goto } from '$app/navigation'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'

	import { sendUserToast } from '$lib/toast'
	import Urlize from '$lib/components/Urlize.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'

	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import InlineCodeCopy from '$lib/components/InlineCodeCopy.svelte'
	import CliHelpBox from '$lib/components/CliHelpBox.svelte'
	import { Archive, ArchiveRestore, FolderOpen, Globe2, Server, Share, Trash } from 'lucide-svelte'
	import { SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB } from '$lib/consts'
	import { scriptToHubUrl } from '$lib/hub'

	let userSettings: UserSettings
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
			hash: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/h/${script?.hash}`,
			path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/p/${script?.path}`
		},
		sync: {
			hash: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/h/${script?.hash}`,
			path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/p/${script?.path}`,
			get_path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/p/${script?.path}`
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
	let scheduleEditor: ScheduleEditor

	let args = undefined
	let moveDrawer: MoveDrawer
	let deploymentDrawer: DeployWorkspaceDrawer

	function getMainButtons(script: Script | undefined) {
		if (!script) return []

		const buttons: any = []

		if (!$userStore?.operator) {
			buttons.push({
				href: `/scripts/edit/${script.path}?args=${encodeState(args)}${
					topHash ? `&hash=${script.hash}&topHash=` + topHash : ''
				}`,
				label: 'Edit',
				buttonProps: {
					disabled: !can_write,
					size: 'xs',
					startIcon: faEdit,
					color: 'dark',
					variant: 'contained'
				}
			})

			if (!topHash) {
				buttons.push({
					href: `/scripts/add?template=${script.path}`,
					label: 'Fork',
					buttonProps: {
						variant: 'border',
						size: 'xs',
						color: 'light',
						startIcon: faCodeFork
					}
				})
			}
		}

		if (Array.isArray(script.parent_hashes) && script.parent_hashes.length > 0) {
			buttons.push({
				label: `(${script.parent_hashes.length})`,
				href: `/scripts/get/${script.parent_hashes[0]}?workspace=${$workspaceStore}`,
				buttonProps: {
					size: 'xs',
					color: 'dark',
					startIcon: faHistory,
					dropdownItems: script.parent_hashes.map((hash) => ({
						href: `/scripts/get/${hash}?workspace=${$workspaceStore}`,
						label: hash
					}))
				}
			})
		}

		return buttons
	}

	function getMenuItems(script: Script | undefined) {
		if (!script) return []

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

					goto(
						scriptToHubUrl(
							script.content,
							script.summary,
							script.description ?? '',
							script.kind,
							script.language,
							script.schema,
							script.language == 'deno' ? '' : script.lock
						).toString()
					)
				}
			})
		}

		if (script.archived) {
			menuItems.push({
				label: 'Unarchive',
				Icon: ArchiveRestore,
				onclick: async () => {
					unarchiveScript(script.path)
				}
			})
		} else {
			menuItems.push({
				label: 'Archive',
				Icon: Archive,
				onclick: async () => {
					archiveScript(script.path)
				}
			})
		}

		menuItems.push({
			label: 'Delete',
			Icon: Trash,
			onclick: async () => {
				deleteScript(script.path)
			}
		})

		return menuItems
	}
</script>

<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/scripts/get/' + e.detail + `?workspace=${$workspaceStore}`)
		loadScript($page.params.hash)
	}}
/>

<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ScheduleEditor bind:this={scheduleEditor} />
<UserSettings bind:this={userSettings} scopes={[`run:script/${script?.path}`]} />
<ShareModal bind:this={shareModal} />

{#if script}
	<DetailPageLayout>
		<svelte:fragment slot="header">
			<DetailPageHeader
				mainButtons={getMainButtons(script)}
				menuItems={getMenuItems(script)}
				summary={script.summary}
				path={script.path}
				edited_at={script.created_at}
				edited_by={script.created_by}
			/>
		</svelte:fragment>
		<svelte:fragment slot="form">
			<div class="p-8 w-full max-w-3xl mx-auto">
				{#if !emptyString(script.description)}
					<div class="border p-2">
						<Urlize text={defaultIfEmptyString(script.description, 'No description')} />
					</div>
				{/if}

				<RunForm
					loading={runLoading}
					autofocus
					detailed={false}
					bind:isValid
					runnable={script}
					runAction={runScript}
					bind:args
					viewCliRun={false}
					schedulable={false}
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
						runForm?.setArgs(JSON.parse(JSON.stringify(e.detail)))
					}}
				/>
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="webhooks">
			<WebhooksPanel
				scopes={[`run:script/${script?.path}`]}
				{webhooks}
				path={script?.path}
				{args}
			/>
		</svelte:fragment>
		<svelte:fragment slot="schedule">
			<div class="p-2 flex flex-col">
				<Button
					on:click={() => scheduleEditor?.openNew(false, script?.path ?? '')}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faCalendar }}
				>
					New Schedule
				</Button>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="details">
			<div>
				<Skeleton {loading} layout={[[20]]} />

				<Tabs selected="code">
					<Tab value="code">Code</Tab>
					<Tab value="dependencies">Dependencies lock file</Tab>
					<Tab value="arguments">
						<span class="inline-flex items-center gap-1">
							Arguments JSON Schema
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
							<div class="p-2">
								<HighlightCode
									language={script.language}
									code={script.content}
									class="whitespace-pre"
								/>
							</div>
						</TabContent>
						<TabContent value="dependencies">
							<div class="">
								{#if script?.lock}
									<pre class="bg-gray-50 text-sm p-2">{script.lock}</pre>
								{:else}
									<p class="bg-gray-50 text-sm p-2"> There is no lock file for this script</p>
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
				<InlineCodeCopy content={cliCommand} />
				<CliHelpBox />
			</div>
		</svelte:fragment>
	</DetailPageLayout>
{/if}
