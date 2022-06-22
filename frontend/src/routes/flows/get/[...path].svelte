<script lang="ts">
	import { page } from '$app/stores'
	import { FlowService, type Flow } from '$lib/gen'
	import { sendUserToast, displayDaysAgo, canWrite } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import {
		faPlay,
		faEdit,
		faArchive,
		faList,
		faCalendar,
		faShare
	} from '@fortawesome/free-solid-svg-icons'
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'

	import github from 'svelte-highlight/styles/github'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'

	let flow: Flow | undefined
	let can_write = false

	let path = $page.params.path
	let shareModal: ShareModal

	$: {
		if ($workspaceStore && $userStore) {
			loadFlow(path)
		}
	}

	async function archiveFlow(hash: string): Promise<void> {
		try {
			await FlowService.archiveFlowByPath({ workspace: $workspaceStore!, path })
			loadFlow(path)
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not archive this flow ${err.body}`, true)
		}
	}

	async function loadFlow(hash: string): Promise<void> {
		flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
		can_write = canWrite(flow.path, flow.extra_perms!, $userStore)
	}
</script>

<svelte:head>
	{@html github}
</svelte:head>

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
								flow?.path && archiveFlow(flow.path)
							},
							disabled: flow.archived || !can_write
						}
					]}
				/>
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
			{#if flow.archived}
				<div class="bg-red-100 border-l-4 border-red-500 text-orange-700 p-4" role="alert">
					<p class="font-bold">Archived</p>
					<p>This version was archived</p>
				</div>
			{/if}

			<div>
				<h3 class="text-gray-700 ">Edited at</h3>
				{displayDaysAgo(flow.edited_at ?? '')}
			</div>
			<div>
				<h3 class="text-gray-700 ">Last editor</h3>
				{flow.edited_by}
			</div>
			<div>
				<h3 class="text-gray-700 ">Summary</h3>
				{flow.summary}
			</div>
			<div>
				<h3 class="text-gray-700 ">Description</h3>
				<div class="prose mt-5">
					<SvelteMarkdown source={flow.description ?? ''} />
				</div>
			</div>
			<div>
				<span>Webhook to run this flow:</span>
				<Tooltip class="font-normal mx-1"
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
				<div class="grid grid-cols-2 gap-4 pb-1 mb-3 border-b">
					<h3 class="text-gray-700 ">
						Arguments JSON schema <Tooltip class="font-normal mx-1"
							>The jsonschema defines the constraints that the payload must respect to be compatible
							with the input parameters of this flow. The UI form is generated automatically from
							the flow jsonschema. See <a href="https://json-schema.org/"
								>jsonschema documentation</a
							></Tooltip
						>
					</h3>
				</div>
				<SchemaViewer schema={flow.schema} />
			</div>
			<div>
				<h3 class="text-gray-700 pb-1 mb-3 border-b">Flow</h3>
				<Highlight language={json} code={JSON.stringify(flow.value, null, 4)} />
			</div>
		{/if}
	</div>
</CenteredPage>
