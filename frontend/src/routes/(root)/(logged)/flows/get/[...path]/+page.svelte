<script lang="ts">
	import { page } from '$app/stores'
	import { FlowService, JobService, ScheduleService, type Flow, type Schedule } from '$lib/gen'
	import {
		canWrite,
		copyToClipboard,
		defaultIfEmptyString,
		emptyString,
		encodeState
	} from '$lib/utils'
	import {
		faChevronDown,
		faChevronUp,
		faClipboard,
		faCodeFork,
		faEdit
	} from '@fortawesome/free-solid-svg-icons'

	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import { goto } from '$app/navigation'
	import { Badge, Button, Skeleton, Tab, TabContent, Tabs } from '$lib/components/common'
	import CronInput from '$lib/components/CronInput.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'
	import { sendUserToast } from '$lib/toast'
	import Urlize from '$lib/components/Urlize.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import { FolderOpen, Globe2, Archive, Trash, Server, Share } from 'lucide-svelte'

	import { flowToHubUrl } from '$lib/hub'
	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'

	let userSettings: UserSettings
	let flow: Flow | undefined
	let schedule: Schedule | undefined
	let can_write = false
	let path = $page.params.path
	let shareModal: ShareModal

	$: {
		if ($workspaceStore && $userStore) {
			loadFlow()
			loadSchedule()
		}
	}

	async function loadSchedule() {
		try {
			let exists = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path
			})
			if (exists) {
				schedule = await ScheduleService.getSchedule({
					workspace: $workspaceStore ?? '',
					path
				})
			}
		} catch (e) {
			console.log('no primary schedule')
		}
	}

	async function archiveFlow(): Promise<void> {
		await FlowService.archiveFlowByPath({
			workspace: $workspaceStore!,
			path,
			requestBody: { archived: !flow?.archived }
		})
		loadFlow()
	}

	async function deleteFlow(): Promise<void> {
		await FlowService.deleteFlowByPath({ workspace: $workspaceStore!, path })
		sendUserToast('Flow deleted')
		goto('/')
	}

	async function setScheduleEnabled(path: string, enabled: boolean): Promise<void> {
		try {
			await ScheduleService.setScheduleEnabled({
				path,
				workspace: $workspaceStore!,
				requestBody: { enabled }
			})
			loadSchedule()
		} catch (err) {
			sendUserToast(`Cannot ` + (enabled ? 'disable' : 'enable') + ` schedule: ${err}`, true)
			loadSchedule()
		}
	}

	async function loadFlow(): Promise<void> {
		flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
		can_write = canWrite(flow.path, flow.extra_perms!, $userStore)
	}

	$: urlAsync = `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/f/${flow?.path}`
	$: urlSync = `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/f/${flow?.path}`

	let isValid = true
	let loading = false

	async function runFlow(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		loading = true
		const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
		let run = await JobService.runFlowByPath({
			workspace: $workspaceStore!,
			path,
			invisibleToOwner,
			requestBody: args,
			scheduledFor
		})
		await goto('/run/' + run + '?workspace=' + $workspaceStore)
	}
	let scheduleEditor: ScheduleEditor

	let viewWebhookCommand = false

	let args = undefined

	function curlCommand(async: boolean) {
		return `curl -H 'Content-Type: application/json' -H "Authorization: Bearer $TOKEN" -X POST -d '${JSON.stringify(
			args
		)}' ${$page.url.protocol}//${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run${
			async ? '' : '_wait_result'
		}/f/${flow?.path}`
	}

	let moveDrawer: MoveDrawer
	let deploymentDrawer: DeployWorkspaceDrawer
	let runForm: RunForm

	function getMainButtons(flow: Flow | undefined) {
		if (!flow) return []

		const buttons: any = []
		if (!$userStore?.operator) {
			buttons.push({
				label: 'Edit',
				buttonProps: {
					href: `/flows/edit/${path}?nodraft=true&args=${encodeState(args)}`,
					variant: 'contained',
					size: 'sm',
					color: 'dark',
					disabled: !can_write,
					startIcon: faEdit
				}
			})

			buttons.push({
				label: 'Fork',
				buttonProps: {
					href: `/flows/add?template=${flow.path}`,
					variant: 'border',
					color: 'light',
					size: 'xs',
					disabled: !can_write,
					startIcon: faCodeFork
				}
			})
		}
		return buttons
	}

	function getMenuItems(flow: Flow | undefined) {
		if (!flow) return []

		const menuItems: any = []
		menuItems.push({
			label: 'Publish to Hub',
			href: flowToHubUrl(flow).toString(),
			Icon: Globe2
		})

		menuItems.push({
			label: 'Share',
			onclick: () => shareModal.openDrawer(flow?.path ?? '', 'flow'),
			Icon: Share,
			disabled: !can_write
		})

		menuItems.push({
			label: 'Move/Rename',
			onclick: () => moveDrawer.openDrawer(flow?.path ?? '', flow?.summary, 'flow'),
			Icon: FolderOpen
		})

		menuItems.push({
			label: 'Deploy to staging/prod',
			onclick: () => deploymentDrawer.openDrawer(flow?.path ?? '', 'flow'),
			Icon: Server
		})

		if (can_write) {
			menuItems.push({
				label: flow.archived ? 'Unarchive' : 'Archive',
				onclick: () => flow?.path && archiveFlow(),
				Icon: Archive
			})
			menuItems.push({
				label: 'Delete',
				onclick: () => flow?.path && deleteFlow(),
				Icon: Trash
			})
		}
		return menuItems
	}
</script>

<Skeleton
	class="!max-w-6xl !px-4 sm:!px-6 md:!px-8"
	loading={!flow}
	layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }], 0.2, [{ h: 1, w: 30 }]]}
/>

<ScheduleEditor on:update={() => loadSchedule()} bind:this={scheduleEditor} />
<UserSettings bind:this={userSettings} scopes={[`run:flow/${flow?.path}`]} />
<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ShareModal bind:this={shareModal} />
<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/flows/get/' + e.detail + `?workspace=${$workspaceStore}`)
		loadFlow()
		loadSchedule()
	}}
/>

{#if flow}
	<DetailPageLayout>
		<svelte:fragment slot="header">
			<DetailPageHeader
				mainButtons={getMainButtons(flow)}
				menuItems={getMenuItems(flow)}
				summary={flow.summary}
				path={flow.path}
				edited_at={flow.edited_at}
				edited_by={flow.edited_by}
				{schedule}
				archived={flow.archived}
			/>
		</svelte:fragment>
		<svelte:fragment slot="form">
			<div class="p-8 w-full max-w-3xl mx-auto">
				{#if !emptyString(flow.description)}
					<div class="box overflow-auto break-words whitespace-pre-wrap">
						<Urlize text={defaultIfEmptyString(flow.description, 'No description')} />
					</div>
				{/if}

				<RunForm
					{loading}
					autofocus
					detailed={false}
					bind:isValid
					runnable={flow}
					runAction={runFlow}
					bind:args
					viewCliRun
					isFlow
					bind:this={runForm}
				/>
			</div>
			<div class="p-4 mx-auto">
				<FlowViewer {flow} noSummary={true} />
			</div>
		</svelte:fragment>
		<svelte:fragment slot="save_inputs">
			<SavedInputs
				flowPath={flow?.path}
				{isValid}
				args={args ?? {}}
				on:selected_args={(e) => {
					runForm?.setArgs(JSON.parse(JSON.stringify(e.detail)))
				}}
			/>
		</svelte:fragment>
		<svelte:fragment slot="webhooks">
			<div class="box max-w-5xl">
				<div class="flex w-full flex-justify-between mb-1">
					<a
						on:click={(e) => {
							e.preventDefault()
							copyToClipboard($page.url.protocol + '//' + urlAsync)
						}}
						href={$page.url.protocol + '//' + urlAsync}
						class="whitespace-nowrap text-ellipsis overflow-hidden mr-1 w-full"
					>
						{urlAsync}
						<span class="text-gray-700 ml-2">
							<Icon data={faClipboard} />
						</span>
					</a>
					<Badge>UUID/Async</Badge>
				</div>
				<div class="mb-2 w-full flex flex-justify-between">
					<a
						on:click={(e) => {
							e.preventDefault()
							copyToClipboard($page.url.protocol + '//' + urlSync)
						}}
						href={$page.url.protocol + '//' + urlSync}
						class="whitespace-nowrap text-ellipsis overflow-hidden mr-1 w-full"
					>
						{urlSync}
						<span class="text-gray-700 ml-2">
							<Icon data={faClipboard} />
						</span>
					</a>
					<Badge>Result/Sync</Badge>
				</div>
				<div class="flex flex-row-reverse">
					<Button size="xs" on:click={userSettings.openDrawer}>
						Create a Webhook-specific Token
						<Tooltip>
							The token will have a scope such that it can only be used to trigger this flow. It is
							safe to share as it cannot be used to impersonate you.
						</Tooltip>
					</Button>
				</div>
				<div class="flex flex-col gap-2 mt-2">
					<div class="flex">
						<Button
							color="light"
							size="lg"
							endIcon={{ icon: viewWebhookCommand ? faChevronUp : faChevronDown }}
							on:click={() => (viewWebhookCommand = !viewWebhookCommand)}
						>
							CURL
						</Button>
					</div>
					{#if viewWebhookCommand}
						<div transition:slide|local class="px-4">
							<Tabs selected="async">
								<Tab value="async">UUID/Async</Tab>
								<Tab value="sync">Result/Sync</Tab>
								<svelte:fragment slot="content">
									<!-- svelte-ignore a11y-click-events-have-key-events -->
									<pre class="bg-gray-700 text-gray-100 p-2 font-mono text-sm whitespace-pre-wrap"
										><TabContent value="async"
											>{curlCommand(true)} <span
												on:click={() => copyToClipboard(curlCommand(true))}
												class="cursor-pointer ml-2"><Icon data={faClipboard} /></span
											><br /><br />//^ returns an UUID. Fetch result until completed == true<br
											/>curl -H "Authorization: Bearer $TOKEN" {$page.url.protocol}//{$page.url
												.hostname}/api/w/{$workspaceStore}/jobs_u/completed/get_result_maybe/$UUID</TabContent
										><TabContent value="sync"
											>{curlCommand(false)} <span
												on:click={() => copyToClipboard(curlCommand(false))}
												class="cursor-pointer ml-2"><Icon data={faClipboard} /></span
											></TabContent
										></pre
									>
								</svelte:fragment>
							</Tabs>
						</div>
					{/if}
				</div>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="schedule">
			{#if schedule}
				<div class="mt-10">
					<h2
						id="primary-schedule"
						class="text-gray-700 pb-1 mb-3 border-b inline-flex flex-row items-center gap-x-4"
						><div>Primary Schedule </div>
						<Badge color="gray">{schedule.schedule}</Badge>
						<Toggle
							checked={schedule.enabled}
							on:change={(e) => {
								if (can_write) {
									setScheduleEnabled(path, e.detail)
								} else {
									sendUserToast('not enough permission', true)
								}
							}}
						/>
						<Button size="xs" on:click={() => scheduleEditor?.openEdit(flow?.path ?? '', true)}
							>Edit schedule</Button
						>
					</h2>
					<div class="max-w-lg">
						<JobArgs args={schedule.args ?? {}} />
					</div>
					<div class="box max-w-5xl mt-2">
						<CronInput disabled={true} schedule={schedule.schedule} timezone={schedule.timezone} />
					</div>
				</div>
			{/if}
		</svelte:fragment>
	</DetailPageLayout>
{/if}
