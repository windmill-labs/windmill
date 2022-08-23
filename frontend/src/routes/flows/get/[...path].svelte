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
		flowToHubUrl
	} from '$lib/utils'
	import Icon from 'svelte-awesome'
	import {
		faPlay,
		faEdit,
		faArchive,
		faList,
		faCalendar,
		faShare,
		faGlobe,
		faCodeFork
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
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'

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
			sendUserToast(`Cannot ` + enabled ? 'disable' : 'enable' + ` schedule: ${err}`, true)
		}
	}

	async function loadFlow(): Promise<void> {
		flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
		can_write = canWrite(flow.path, flow.extra_perms!, $userStore)
	}
</script>

<CenteredPage>
	<div class="flex flex-row justify-between">
		<h1>
			<a href="/flows/get/{path}">{flow?.path ?? 'Loading...'}</a>

			<SharedBadge canWrite={can_write} extraPerms={flow?.extra_perms ?? {}} />
		</h1>

		{#if flow}
			<div class="flex flex-row-reverse px-6">
				<Dropdown
					dropdownItems={[
						{
							displayName: 'Use as template',
							icon: faEdit,
							href: `/flows/add?template=${flow.path}`
						},
						{
							displayName: 'Share',
							icon: faShare,
							action: () => {
								shareModal.openModal()
							},
							disabled: !can_write
						},
						{
							displayName: 'Schedule',
							icon: faCalendar,
							href: `/schedule/add?path=${flow.path}&isFlow=true`
						},
						{
							displayName: 'Archive',
							icon: faArchive,
							type: 'delete',
							action: () => {
								flow?.path && archiveFlow()
							},
							disabled: flow.archived || !can_write
						}
					]}
				/>
				<div class="px-1">
					<a
						target="_blank"
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href={flowToHubUrl(flow).toString()}
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faGlobe} scale={0.5} />
							<span class="pl-1">Publish to Hub</span>
						</div>
					</a>
				</div>
				<div class="px-1">
					<a
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href="/runs/{flow.path}"
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faList} scale={0.5} />
							<span class="pl-1">View runs</span>
						</div>
					</a>
				</div>
				<div class="px-1">
					<a
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href="/flows/edit/{path}"
						class:disabled={!can_write}
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faEdit} scale={0.5} />
							<span class="pl-1">Edit</span>
						</div>
					</a>
					<a
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href="/flows/add?template={flow.path}"
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faCodeFork} scale={0.5} />
							<span class="pl-1">Use as template/Fork</span>
						</div>
					</a>
				</div>
				<div class="px-1">
					<a
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href="/flows/run/{path}"
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faPlay} scale={0.5} />
							<span class="pl-1">Run</span>
						</div>
					</a>
				</div>
			</div>
		{/if}
	</div>

	<ShareModal bind:this={shareModal} kind="flow" path={flow?.path ?? ''} />

	<div class="grid grid-cols-1 gap-6 max-w-7xl pb-6">
		{#if flow === undefined}
			<p>loading</p>
		{:else}
			<p class="text-sm">Edited {displayDaysAgo(flow.edited_at ?? '')} by {flow.edited_by}</p>
			<h2>{flow.summary}</h2>

			<div class="prose">
				<SvelteMarkdown source={defaultIfEmptyString(flow.description, 'No description')} />
			</div>
			{#if schedule}
				<div>
					<h2 class="text-gray-700 pb-1 mb-3 border-b">Primary Schedule</h2>
					<div>
						<h3 class="text-gray-700 ">Enabled</h3>
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
					</div>
					<div>
						<div>
							<h3 class="text-gray-700">Schedule</h3>
							<span class="font-mono p-1 border" class:bg-gray-300={!schedule.enabled}
								>{schedule.schedule}</span
							>
						</div>
						<div>
							<h3 class="text-gray-700 ">Args</h3>
							<ObjectViewer json={schedule.args ?? {}} pureViewer={true} />
						</div>
					</div>
				</div>
			{/if}
			{#if flow.archived}
				<div class="bg-red-100 border-l-4 border-red-500 text-orange-700 p-4" role="alert">
					<p class="font-bold">Archived</p>
					<p>This version was archived</p>
				</div>
			{/if}
			<div>
				<span>Webhook to run this flow:</span>
				<Tooltip
					>Send a POST http request with a token as bearer token and the args respecting the
					corresponding jsonschema as payload. To create a permanent token, go to your user setting
					by clicking your username on the top-left.</Tooltip
				>
				<pre><code
						><a href="/api/w/{$workspaceStore}/jobs/run/f/{flow?.path}"
							>/api/w/{$workspaceStore}/jobs/run/f/{flow?.path}</a
						></code
					></pre>
			</div>
			<div>
				<h2 class="text-gray-700 pb-1 mb-3 border-b">Flow</h2>
				<FlowViewer {flow} />
			</div>
		{/if}
	</div>
</CenteredPage>
