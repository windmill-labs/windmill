<script lang="ts">
	import { page } from '$app/stores'
	import { FlowService, JobService, ScheduleService, type Flow, type Schedule } from '$lib/gen'
	import {
		canWrite,
		defaultIfEmptyString,
		displayDaysAgo,
		emptyString,
		encodeState
	} from '$lib/utils'
	import { faCalendar, faCodeFork, faEdit } from '@fortawesome/free-solid-svg-icons'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import { goto } from '$app/navigation'
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import Urlize from '$lib/components/Urlize.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import { FolderOpen, Archive, Trash, Server, Share, PenBox, ListOrdered } from 'lucide-svelte'

	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import CliHelpBox from '$lib/components/CliHelpBox.svelte'
	import InlineCodeCopy from '$lib/components/InlineCodeCopy.svelte'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'

	let flow: Flow | undefined
	let schedule: Schedule | undefined
	let can_write = false
	let path = $page.params.path
	let shareModal: ShareModal

	$: cliCommand = `wmill flow run ${flow?.path} -d '${JSON.stringify(args)}'`

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

			sendUserToast(`Schedule ${enabled ? 'enabled' : 'disabled'}`)
		} catch (err) {
			sendUserToast(`Cannot ` + (enabled ? 'disable' : 'enable') + ` schedule: ${err}`, true)
			loadSchedule()
		}
	}

	async function loadFlow(): Promise<void> {
		flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
		can_write = canWrite(flow.path, flow.extra_perms!, $userStore)
	}

	$: urlAsync = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/f/${flow?.path}`
	$: urlSync = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run_wait_result/f/${flow?.path}`

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

	let args = undefined

	let moveDrawer: MoveDrawer
	let deploymentDrawer: DeployWorkspaceDrawer
	let runForm: RunForm

	function getMainButtons(flow: Flow | undefined) {
		if (!flow || $userStore?.operator) return []

		const buttons: any = []
		if (!$userStore?.operator) {
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
		}
		return buttons
	}

	function getMenuItems(flow: Flow | undefined) {
		if (!flow || $userStore?.operator) return []

		const menuItems: any = []

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
				Icon: Archive,
				color: 'red'
			})
			menuItems.push({
				label: 'Delete',
				onclick: () => flow?.path && deleteFlow(),
				Icon: Trash,
				color: 'red'
			})
		}
		return menuItems
	}

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

<Skeleton
	class="!max-w-6xl !px-4 sm:!px-6 md:!px-8"
	loading={!flow}
	layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }], 0.2, [{ h: 1, w: 30 }]]}
/>
<svelte:window on:keydown={onKeyDown} />
<ScheduleEditor on:update={() => loadSchedule()} bind:this={scheduleEditor} />
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
	<DetailPageLayout isOperator={$userStore?.operator}>
		<svelte:fragment slot="header">
			<DetailPageHeader
				mainButtons={getMainButtons(flow)}
				menuItems={getMenuItems(flow)}
				title={defaultIfEmptyString(flow.summary, flow.path)}
			/>
		</svelte:fragment>
		<svelte:fragment slot="form">
			<SplitPanesWrapper>
				<Splitpanes horizontal>
					<Pane size={60} minSize={20}>
						<div class="p-8 w-full max-w-3xl mx-auto gap-2">
							<div class="flex flex-col gap-0.5">
								{#if !emptyString(flow.summary)}
									<span class="text-lg font-semibold">{flow.path}</span>
								{/if}
								<span class="text-sm text-gray-600">
									Edited {displayDaysAgo(flow.edited_at ?? '')} by {flow.edited_by}
								</span>

								{#if flow.archived}
									<div class="" />
									<Alert type="error" title="Archived">This flow was archived</Alert>
								{/if}

								{#if !emptyString(flow.description)}
									<div class=" break-words whitespace-pre-wrap text-sm">
										<Urlize text={defaultIfEmptyString(flow.description, 'No description')} />
									</div>
								{/if}
							</div>

							<RunForm
								viewKeybinding
								{loading}
								autofocus
								detailed={false}
								bind:isValid
								runnable={flow}
								runAction={runFlow}
								bind:args
								isFlow
								bind:this={runForm}
							/>
						</div>
					</Pane>
					<Pane size={40} minSize={20}>
						<FlowGraphViewer download {flow} overflowAuto noSide={true} />
					</Pane>
				</Splitpanes>
			</SplitPanesWrapper>
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
		<svelte:fragment slot="details">
			<div class="p-1">
				<SchemaViewer schema={flow.schema} />
			</div>
		</svelte:fragment>
		<svelte:fragment slot="webhooks">
			<WebhooksPanel
				scopes={[`run:flow/${flow?.path}`]}
				webhooks={{
					async: {
						path: urlAsync
					},
					sync: {
						path: urlSync
					}
				}}
				isFlow={true}
				{args}
			/>
		</svelte:fragment>
		<svelte:fragment slot="schedule">
			<div class="flex flex-row justify-end p-2">
				<Button
					on:click={() => scheduleEditor?.openNew(true, flow?.path ?? '')}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faCalendar }}
				>
					New Schedule
				</Button>
			</div>
			{#if schedule}
				<div class="p-2 flex flex-col gap-2">
					<div class="flex flex-row justify-between h-8">
						<div class="flex flex-row gap-2">
							<input
								class="inline-block !w-32"
								type="text"
								id="cron-schedule"
								name="cron-schedule"
								placeholder="*/30 * * * *"
								value={schedule.schedule}
								disabled={true}
							/>
							<Badge color="indigo" small>Primary schedule</Badge>
						</div>
						<div class="flex flex-row gap-2">
							<Toggle
								checked={schedule.enabled}
								on:change={(e) => {
									if (can_write) {
										setScheduleEnabled(path, e.detail)
									} else {
										sendUserToast('not enough permission', true)
									}
								}}
								options={{
									right: 'On'
								}}
								size="xs"
							/>
							<Button size="xs" variant="border" color="light" href={`/runs/${flow?.path}`}>
								<div class="flex flex-row gap-2">
									<ListOrdered size={14} />
									Runs
								</div>
							</Button>
							<Button
								size="xs"
								color="dark"
								on:click={() => scheduleEditor?.openEdit(flow?.path ?? '', true)}
							>
								<PenBox size={14} />
							</Button>
						</div>
					</div>
					{#if Object.keys(schedule?.args ?? {}).length > 0}
						<div class="">
							<JobArgs args={schedule.args ?? {}} />
						</div>
					{:else}
						<div class="text-xs texg-gray-700"> This flow takes no argument </div>
					{/if}
				</div>
			{/if}
		</svelte:fragment>
		<svelte:fragment slot="cli">
			<div class="p-2">
				<InlineCodeCopy content={cliCommand} />
				<CliHelpBox />
			</div>
		</svelte:fragment>
	</DetailPageLayout>
{/if}
