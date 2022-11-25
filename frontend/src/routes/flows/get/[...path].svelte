<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Flow ${params.path}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { FlowService, ScheduleService, type Flow, type Schedule } from '$lib/gen'
	import {
		displayDaysAgo,
		canWrite,
		sendUserToast,
		defaultIfEmptyString,
		flowToHubUrl,
		copyToClipboard,
		emptyString
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
		faClipboard
	} from '@fortawesome/free-solid-svg-icons'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { Button, ActionRow, Skeleton, Badge } from '$lib/components/common'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import CronInput from '$lib/components/CronInput.svelte'
	import Icon from 'svelte-awesome'

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
			schedule = await ScheduleService.getSchedule({
				workspace: $workspaceStore ?? '',
				path
			})
		} catch (e) {
			console.log('no primary schedule')
		}
	}

	async function archiveFlow(): Promise<void> {
		await FlowService.archiveFlowByPath({ workspace: $workspaceStore!, path })
		loadFlow()
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

	$: url = `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/f/${flow?.path}`
</script>

<UserSettings bind:this={userSettings} />

<Skeleton
	class="!max-w-6xl !px-4 sm:!px-6 md:!px-8"
	loading={!flow}
	layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }], 0.2, [{ h: 1, w: 30 }]]}
/>
{#if flow}
	<ActionRow applyPageWidth>
		<svelte:fragment slot="left">
			<Button
				on:click={() => flow?.path && archiveFlow()}
				variant="border"
				color="red"
				size="xs"
				startIcon={{ icon: faArchive }}
				disabled={flow.archived || !can_write}
			>
				Archive
			</Button>
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
				href="/schedule/add?path={flow.path}&isFlow=true"
				variant="border"
				color="light"
				size="xs"
				startIcon={{ icon: faCalendar }}
			>
				Schedule
			</Button>
			<Button
				on:click={() => shareModal.openDrawer()}
				variant="border"
				color="light"
				size="xs"
				startIcon={{ icon: faShare }}
				disabled={!can_write}
			>
				Share
			</Button>
		</svelte:fragment>
		<svelte:fragment slot="right">
			<Button
				href="/flows/run/{path}"
				variant="contained"
				color="blue"
				size="xs"
				startIcon={{ icon: faPlay }}
			>
				Run
			</Button>
			<Button
				href="/flows/edit/{path}"
				variant="contained"
				color="blue"
				size="xs"
				startIcon={{ icon: faEdit }}
				disabled={!can_write}
			>
				Edit
			</Button>
			<Button
				href="/flows/add?template={flow.path}"
				variant="contained"
				color="blue"
				size="xs"
				startIcon={{ icon: faCodeFork }}
			>
				Use as template/Fork
			</Button>
			<Button href="/runs/{flow.path}" color="blue" size="xs" startIcon={{ icon: faList }}>
				View runs
			</Button>
		</svelte:fragment>
	</ActionRow>
{/if}

<CenteredPage>
	{#if flow}
		<h1 class="break-words py-2 mr-2">
			{defaultIfEmptyString(flow.summary, flow.path)}
		</h1>
		{#if !emptyString(flow.summary)}
			<h2 class="font-bold pb-4">{flow.path}</h2>
		{/if}
	{/if}
	<ShareModal bind:this={shareModal} kind="flow" path={flow?.path ?? ''} />

	<div class="grid grid-cols-1 gap-6 max-w-7xl pb-6">
		<Skeleton
			loading={!flow}
			layout={[[{ h: 1.5, w: 40 }], 1, [4], 2.25, [{ h: 1.5, w: 30 }], 1, [10]]}
		/>
		{#if flow}
			<p class="text-sm text-gray-600"
				>Edited {displayDaysAgo(flow.edited_at ?? '')} by {flow.edited_by}
				<a href="#webhook" class="ml-2">
					<Badge color="dark-blue">Webhook</Badge>
				</a></p
			>

			{#if flow.archived}
				<div class="bg-red-100 border-l-4 border-red-500 text-orange-700 p-4" role="alert">
					<p class="font-bold">Archived</p>
					<p>This version was archived</p>
				</div>
			{/if}
			<div class="prose text-sm box max-w-6xl w-full mt-4">
				<SvelteMarkdown source={defaultIfEmptyString(flow?.description, 'No description')} />
			</div>
			<div class="mt-4">
				<FlowViewer {flow} noSummary={true} />

				<h2 id="webhook" class="mt-10 text-gray-700 pb-1 mb-3 border-b"
					>Webhook<Tooltip
						>To trigger this script with a webhook, do a POST request to the endpoint below. Flows
						are not public and can only be run by users with at least view rights on them. You will
						need to pass a bearer token to authentify as a user. You can either pass it as a Bearer
						token or as query arg `?token=XXX`. <a
							href="https://docs.windmill.dev/docs/getting_started/webhooks">See docs</a
						></Tooltip
					></h2
				>
				<div class="box max-w-2xl">
					<div class="flex flex-row gap-x-2 w-full">
						<a
							on:click={(e) => {
								e.preventDefault()
								copyToClipboard(url)
							}}
							href={$page.url.protocol + '//' + url}
							class="whitespace-nowrap text-ellipsis overflow-hidden mr-1"
						>
							{url}
							<span class="text-gray-700 ml-2">
								<Icon data={faClipboard} />
							</span>
						</a>

						<Button size="xs" on:click={userSettings.openDrawer}>Create token</Button>
					</div>
				</div>
				{#if schedule}
					<div class="mt-10">
						<h2 class="text-gray-700 pb-1 mb-3 border-b inline-flex flex-row items-center gap-x-4"
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
							<Button size="xs" href="/schedule/add?edit={flow.path}&isFlow=true"
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
		{/if}
	</div>
</CenteredPage>
