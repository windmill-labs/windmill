<script lang="ts">
	import { page } from '$app/stores'
	import { FlowService, JobService, ScheduleService, type Flow, type Schedule } from '$lib/gen'
	import {
		displayDaysAgo,
		canWrite,
		sendUserToast,
		defaultIfEmptyString,
		flowToHubUrl,
		copyToClipboard,
		emptyString,
		encodeState
	} from '$lib/utils'
	import {
		faPlay,
		faEdit,
		faArchive,
		faList,
		faCalendar,
		faShare,
		faGlobe,
		faCodeFork,
		faClipboard,
		faChevronUp,
		faChevronDown,
		faTrash
	} from '@fortawesome/free-solid-svg-icons'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { Button, ActionRow, Skeleton, Badge } from '$lib/components/common'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import CronInput from '$lib/components/CronInput.svelte'
	import Icon from 'svelte-awesome'
	import RunForm from '$lib/components/RunForm.svelte'
	import { goto } from '$app/navigation'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import { slide } from 'svelte/transition'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'

	let userSettings: UserSettings

	let flow: Flow | undefined
	let schedule: Schedule | undefined
	let can_write = false

	let path = $page.params.path
	let shareModal: ShareModal

	let queryId = $page.url.searchParams.get('workspace_id')
	if (queryId && queryId != $workspaceStore) {
		$workspaceStore = $page.url.searchParams.get('workspace_id')!
	}

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
		await FlowService.archiveFlowByPath({ workspace: $workspaceStore!, path })
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

	let runForm: RunForm | undefined
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
	$: curlCommand = `curl -H 'Content-Type: application/json' -H "Authorization: Bearer $TOKEN" -X POST -d '${JSON.stringify(
		args
	)}' ${$page.url.protocol}//${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/f/${
		flow?.path
	}`

	let webhook: HTMLHeadElement
	let moveDrawer: MoveDrawer
</script>

<ScheduleEditor on:update={() => loadSchedule()} bind:this={scheduleEditor} />

<UserSettings bind:this={userSettings} />

<Skeleton
	class="!max-w-6xl !px-4 sm:!px-6 md:!px-8"
	loading={!flow}
	layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }], 0.2, [{ h: 1, w: 30 }]]}
/>

<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/flows/get/' + e.detail + `?workspace_id=${$workspaceStore}`)
		loadFlow()
		loadSchedule()
	}}
/>

<CenteredPage>
	{#if flow}
		<div class="prose-sm mx-auto mt-6">
			<div
				class="flex flex-row-reverse w-full flex-wrap md:flex-nowrap justify-between gap-x-2 gap-y-4"
			>
				<div class="flex flex-row-reverse gap-2 h-full">
					<Button
						href="/flows/run/{path}"
						variant="contained"
						color="blue"
						size="md"
						startIcon={{ icon: faPlay }}
					>
						Run
					</Button>
					{#if !$userStore?.operator}
						<Button
							href="/flows/edit/{path}?nodraft=true&args={encodeState(args)}"
							variant="contained"
							color="blue"
							size="md"
							startIcon={{ icon: faEdit }}
							disabled={!can_write}
						>
							Edit
						</Button>
						<Button
							href="/flows/add?template={flow.path}"
							variant="border"
							color="blue"
							size="md"
							startIcon={{ icon: faCodeFork }}
						>
							Fork
						</Button>
					{/if}
					<Button
						href="/runs/{flow.path}"
						variant="border"
						color="blue"
						size="md"
						startIcon={{ icon: faList }}
					>
						View runs
					</Button>
				</div>
				<h1 class="mb-1 truncate grow">
					{defaultIfEmptyString(flow.summary, flow.path)}
				</h1>
			</div>
			{#if !emptyString(flow.summary)}
				<span class="text-lg font-semibold">{flow.path}</span>
			{/if}
		</div>
	{/if}
	<ShareModal bind:this={shareModal} />

	<div class="grid grid-cols-1 gap-6 max-w-7xl pb-6">
		<Skeleton
			loading={!flow}
			layout={[[{ h: 1.5, w: 40 }], 1, [4], 2.25, [{ h: 1.5, w: 30 }], 1, [10]]}
		/>
		{#if flow}
			<div>
				<span class="text-sm text-gray-600">
					Edited {displayDaysAgo(flow.edited_at ?? '')} by {flow.edited_by}

					{#if schedule}
						<a href="#primary-schedule" class="ml-2">
							<Badge color="dark-blue">Primary schedule</Badge>
						</a>{/if}</span
				>

				{#if flow.archived}
					<div class="bg-red-100 border-l-4 border-red-500 text-orange-700 p-4" role="alert">
						<p class="font-bold">Archived</p>
						<p>This flow was archived</p>
					</div>
				{/if}

				<div class="flex gap-2 flex-wrap mt-2">
					<Button
						target="_blank"
						href={flowToHubUrl(flow).toString()}
						variant="border"
						color="light"
						size="xs"
						startIcon={{ icon: faGlobe }}
					>
						Publish to Hub
					</Button>
					<Button
						on:click={() => shareModal.openDrawer(flow?.path ?? '', 'flow')}
						variant="border"
						color="light"
						size="xs"
						startIcon={{ icon: faShare }}
						disabled={!can_write}
					>
						Share
					</Button>
					<Button
						on:click={() => scheduleEditor?.openNew(true, flow?.path ?? '')}
						variant="border"
						color="light"
						size="xs"
						startIcon={{ icon: faCalendar }}
					>
						Schedule
					</Button>
					<Button
						on:click={() => moveDrawer.openDrawer(flow?.path ?? '', flow?.summary, 'flow')}
						variant="border"
						color="light"
						size="xs"
						startIcon={{ icon: faEdit }}
					>
						Move/Rename
					</Button>
					<Button
						btnClasses="ml-2"
						variant="border"
						size="xs"
						on:click={() => webhook.scrollIntoView()}>Webhook</Button
					>
				</div>
			</div>
			<div class="mt-6 grid grid-cols-1 sm:grid-cols-3 gap-6">
				<div class="col-span-2 box">
					<RunForm
						{loading}
						autofocus
						detailed={false}
						bind:isValid
						bind:this={runForm}
						runnable={flow}
						runAction={runFlow}
						bind:args
						viewCliRun
						isFlow
					/>
				</div>
				{#if !emptyString(flow.description)}
					<div class="box">
						{defaultIfEmptyString(flow.description, 'No description')}
					</div>
				{/if}
			</div>
			<div class="mt-4">
				<FlowViewer {flow} noSummary={true} />

				<h2 bind:this={webhook} class="mt-10 text-gray-700 pb-1 mb-3 border-b"
					>Webhook<Tooltip>
						Pass the input as a json payload, the token as a Bearer token or as query arg
						`?token=XXX` and pass as header: 'Content-Type: application/json <a
							href="https://docs.windmill.dev/docs/core_concepts/webhooks">See docs</a
						></Tooltip
					></h2
				>
				<div class="box max-w-2xl">
					<div class="flex w-full flex-justify-between mb-1">
						<a
							on:click={(e) => {
								e.preventDefault()
								copyToClipboard(urlAsync)
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
								copyToClipboard(urlSync)
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
						<Button size="xs" on:click={userSettings.openDrawer}>Create token</Button>
					</div>
				</div>

				<div class="flex flex-col gap-2 mt-2">
					<div>
						<Button
							color="light"
							size="sm"
							endIcon={{ icon: viewWebhookCommand ? faChevronUp : faChevronDown }}
							on:click={() => (viewWebhookCommand = !viewWebhookCommand)}
						>
							See example curl command
						</Button>
					</div>
					{#if viewWebhookCommand}
						<div transition:slide|local class="px-4">
							<pre class="bg-gray-700 text-gray-100 p-2  font-mono text-sm whitespace-pre-wrap"
								>{curlCommand} <span
									on:click={() => copyToClipboard(curlCommand)}
									class="cursor-pointer ml-2"><Icon data={faClipboard} /></span
								></pre
							>
						</div>
					{/if}
				</div>
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
						<div class="box max-w-lg mt-2">
							<CronInput disabled={true} schedule={schedule.schedule} />
						</div>
					</div>
				{/if}
			</div>
			<div>
				{#if can_write}
					<h2 class="mt-4">Danger zone</h2>
					<div class="flex gap-2 mt-6">
						<Button
							on:click={() => flow?.path && archiveFlow()}
							variant="border"
							color="red"
							size="md"
							startIcon={{ icon: faArchive }}
							disabled={flow.archived || !can_write}
						>
							Archive
						</Button>
						<Button
							on:click={() => flow?.path && deleteFlow()}
							variant="border"
							color="red"
							size="md"
							startIcon={{ icon: faTrash }}
							disabled={flow.archived || !can_write}
						>
							Delete
						</Button>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</CenteredPage>
