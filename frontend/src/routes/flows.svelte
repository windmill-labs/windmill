<script context="module">
	export function load() {
		return {
			stuff: { title: 'Flows' }
		}
	}
</script>

<script lang="ts">
	import { FlowService, type OpenFlow } from '$lib/gen'
	import type { Flow } from '$lib/gen'

	import { sendUserToast, canWrite } from '$lib/utils'
	import {
		faArchive,
		faBuilding,
		faCalendarAlt,
		faCodeFork,
		faEdit,
		faEye,
		faGlobe,
		faList,
		faPlay,
		faShare
	} from '@fortawesome/free-solid-svg-icons'

	import Dropdown from '$lib/components/Dropdown.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { Button, Tabs, Tab, Skeleton, Badge } from '../lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import CreateActions from '$lib/components/flows/CreateActions.svelte'
	import Icon from 'svelte-awesome'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import PickHubFlow from './PickHubFlow.svelte'

	type Tab = 'hub' | 'workspace'
	type FlowW = Flow & { canWrite: boolean; marked?: string }
	let flows: FlowW[] = []
	let filteredFlows: FlowW[] = []
	let flowFilter = ''
	let tab: Tab = 'workspace'
	let shareModal: ShareModal

	let flowViewer: Drawer
	let flowViewerFlow: { flow?: OpenFlow & { id?: number } } | undefined

	let loading = true

	async function loadFlows(): Promise<void> {
		flows = (await FlowService.listFlows({ workspace: $workspaceStore! })).map((x: Flow) => {
			return {
				canWrite: canWrite(x.path, x.extra_perms, $userStore) && x.workspace_id == $workspaceStore,
				...x
			}
		})
		loading = false
	}

	async function archiveFlow(path: string): Promise<void> {
		try {
			await FlowService.archiveFlowByPath({ workspace: $workspaceStore!, path })
			loadFlows()
			sendUserToast(`Successfully archived flow ${path}`)
		} catch (err) {
			sendUserToast(`Could not archive this flow ${err.body}`, true)
		}
	}

	async function viewFlow(obj: { flow_id: number }): Promise<void> {
		// console.log(obj)

		const hub = await FlowService.getHubFlowById({ id: obj.flow_id })
		flowViewerFlow = hub
		flowViewer.openDrawer()
	}

	$: owners = Array.from(
		new Set(filteredFlows?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	$: preFilteredFlows =
		ownerFilter != undefined ? flows.filter((x) => x.path.startsWith(ownerFilter ?? '')) : flows

	let ownerFilter: string | undefined = undefined

	$: {
		if ($workspaceStore && ($userStore || $superadmin)) {
			loadFlows()
		}
	}
</script>

<SearchItems
	filter={flowFilter}
	items={preFilteredFlows}
	bind:filteredItems={filteredFlows}
	f={(x) => x.summary + ' (' + x.path + ')'}
/>

<Drawer bind:this={flowViewer} size="900px">
	<DrawerContent title="Hub flow" on:close={flowViewer.closeDrawer}>
		<div slot="submission" class="flex flex-row gap-2 pr-2"
			><Button
				href="https://hub.windmill.dev/flows/{flowViewerFlow?.flow?.id}"
				startIcon={{ icon: faGlobe }}
				variant="border">View on the Hub</Button
			><Button href="/flows/add?hub={flowViewerFlow?.flow?.id}" startIcon={{ icon: faCodeFork }}
				>Fork</Button
			></div
		>

		{#if flowViewerFlow?.flow}
			<FlowViewer flow={flowViewerFlow.flow} />
		{/if}
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader title="Flows" tooltip="Flows can compose and chain scripts together">
		<div class="flex flex-row">
			<CreateActions />
		</div>
	</PageHeader>

	<Tabs bind:selected={tab}>
		<Tab size="xl" value="workspace"><Icon data={faBuilding} class="mr-1" /> Workspace</Tab>
		<Tab size="xl" value="hub"><Icon data={faGlobe} class="mr-1" /> Hub</Tab>
	</Tabs>

	<div class="mb-1" />
	{#if tab != 'hub'}
		<input type="text" placeholder="Search Flows" bind:value={flowFilter} class="text-2xl mt-2" />

		<div class="gap-2 w-full flex flex-wrap pb-1 pt-2">
			{#each owners as owner}
				<Badge
					class="cursor-pointer hover:bg-gray-200"
					on:click={() => {
						ownerFilter = ownerFilter == owner ? undefined : owner
					}}
					color={owner === ownerFilter ? 'blue' : 'gray'}
				>
					{owner}
					{#if owner === ownerFilter}&cross;{/if}
				</Badge>
			{/each}
		</div>
	{/if}

	{#if tab == 'workspace'}
		<div class="grid grid-cols-1 gap-4 lg:grid-cols-2 mt-2 w-full">
			{#if !loading}
				{#each filteredFlows as { summary, path, extra_perms, canWrite, marked }}
					<a
						class="border border-gray-400 p-2 rounded-sm shadow-sm hover:border-blue-600 text-gray-800 flex flex-row items-center justify-between"
						href="/flows/get/{path}"
					>
						<div class="flex flex-col gap-1 w-full h-full">
							<div class="font-semibold text-gray-700 truncate">
								{#if marked}
									{@html marked}
								{:else}
									{!summary || summary.length == 0 ? path : summary}
								{/if}
							</div>
							<div class="flex flex-row  justify-between w-full grow gap-2 items-start">
								<div class="text-gray-700 text-xs flex flex-row  flex-wrap  gap-x-1 items-center"
									>{path}
									<SharedBadge {canWrite} extraPerms={extra_perms} />
								</div>
								<div class="flex flex-row-reverse place gap-x-2 pt-4">
									<div>
										<Dropdown
											dropdownItems={[
												{
													displayName: 'View flow',
													icon: faEye,
													href: `/flows/get/${path}`
												},
												{
													displayName: 'Edit',
													icon: faEdit,
													href: `/flows/edit/${path}`,
													disabled: !canWrite
												},
												{
													displayName: 'Use as template/Fork',
													icon: faCodeFork,
													href: `/flows/add?template=${path}`
												},
												{
													displayName: 'View runs',
													icon: faList,
													href: `/runs/${path}`
												},
												{
													displayName: 'Schedule',
													icon: faCalendarAlt,
													href: `/schedule/add?path=${path}&isFlow=true`
												},
												{
													displayName: 'Share',
													icon: faShare,
													action: () => {
														shareModal.openDrawer(path)
													},
													disabled: !canWrite
												},
												{
													displayName: 'Archive',
													icon: faArchive,
													action: () => {
														path ? archiveFlow(path) : null
													},
													type: 'delete',
													disabled: !canWrite
												}
											]}
										/>
									</div>
									<div>
										<Button
											color="dark"
											size="xs"
											href="/flows/run/{path}"
											startIcon={{ icon: faPlay }}>Run</Button
										>
									</div>
									{#if canWrite}
										<div>
											<Button
												color="dark"
												variant="border"
												size="xs"
												href="/flows/edit/{path}"
												startIcon={{ icon: faEdit }}
											>
												Edit
											</Button>
										</div>
									{:else}
										<div>
											<Button
												color="dark"
												variant="border"
												size="xs"
												href="/flows/add?template={path}"
												startIcon={{ icon: faCodeFork }}
											>
												Fork
											</Button>
										</div>
									{/if}
								</div>
							</div></div
						></a
					>
				{/each}
			{:else}
				{#each Array(10).fill(0) as sk}
					<Skeleton layout={[[4]]} />
				{/each}
			{/if}
		</div>
	{:else}
		<PickHubFlow on:pick={(e) => viewFlow(e.detail)} />
	{/if}
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	kind="flow"
	on:change={() => {
		loadFlows()
	}}
/>
