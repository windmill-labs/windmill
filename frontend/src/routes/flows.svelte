<script context="module">
	export function load() {
		return {
			stuff: { title: 'Flows' }
		}
	}
</script>

<script lang="ts">
	import Fuse from 'fuse.js'
	import { FlowService, type OpenFlow } from '$lib/gen'
	import type { Flow } from '$lib/gen'

	import { sendUserToast, groupBy, canWrite, loadHubFlows } from '$lib/utils'
	import {
		faArchive,
		faCalendarAlt,
		faCodeFork,
		faEdit,
		faEye,
		faList,
		faPlay,
		faPlus,
		faShare
	} from '@fortawesome/free-solid-svg-icons'

	import Dropdown from '$lib/components/Dropdown.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { Button, Tabs, Tab, Skeleton } from '../lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import CreateActions from '$lib/components/flows/CreateActions.svelte'

	type Tab = 'all' | 'personal' | 'groups' | 'shared' | 'hub'
	type Section = [string, FlowW[]]
	type FlowW = Flow & { canWrite: boolean; tab: Tab }
	let flows: FlowW[] = []
	let filteredFlows: FlowW[]
	let hubFlows: any[] | undefined = undefined
	let filteredHubFlows: any[]
	let flowFilter = ''
	let groupedFlows: Section[] = []
	let hubFilter = ''
	let tab: Tab = 'all'
	let shareModal: ShareModal
	let loading = {
		general: true,
		hub: true
	}

	const flowFuseOptions = {
		includeScore: false,
		keys: ['description', 'path', 'content', 'hash', 'summary']
	}
	const flowFuse: Fuse<FlowW> = new Fuse(flows, flowFuseOptions)

	const flowHubFuse: Fuse<any> = new Fuse([], {
		includeScore: false,
		keys: ['summary']
	})

	$: filteredFlows =
		flowFilter.length > 0 ? flowFuse.search(flowFilter).map((value) => value.item) : flows

	$: filteredHubFlows =
		hubFilter.length > 0 ? flowHubFuse.search(hubFilter).map((value) => value.item) : hubFlows ?? []

	let flowViewer: Drawer
	let flowViewerFlow: OpenFlow | undefined

	$: {
		let defaults: string[] = []

		if (tab == 'all' || tab == 'personal') {
			defaults = defaults.concat(`u/${$userStore?.username}`)
		}
		if (tab == 'all' || tab == 'groups') {
			defaults = defaults.concat($userStore?.groups.map((x) => `g/${x}`) ?? [])
		}
		groupedFlows = groupBy(
			filteredFlows,
			(sc: Flow) => sc.path.split('/').slice(0, 2).join('/'),
			defaults
		)
	}

	function tabFromPath(path: string) {
		let t: Tab = 'shared'
		let path_prefix = path.split('/').slice(0, 2)
		if (path_prefix[0] == 'u' && path_prefix[1] == $userStore?.username) {
			t = 'personal'
		} else if (path_prefix[0] == 'g' && $userStore?.groups.includes(path_prefix[1])) {
			t = 'groups'
		}
		return t
	}

	async function loadFlows(): Promise<void> {
		const allFlows = (await FlowService.listFlows({ workspace: $workspaceStore! })).map(
			(x: Flow) => {
				let t: Tab = tabFromPath(x.path)
				return {
					canWrite:
						canWrite(x.path, x.extra_perms, $userStore) && x.workspace_id == $workspaceStore,
					tab: t,
					...x
				}
			}
		)
		flows = tab == 'all' ? allFlows : allFlows.filter((x) => x.tab == tab)
		loading.general = false
		flowFuse.setCollection(flows)
	}

	async function loadHubFlowsWFuse(): Promise<void> {
		hubFlows = await loadHubFlows()
		loading.hub = false
		flowHubFuse.setCollection(hubFlows ?? [])
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

	async function viewFlow(id: number): Promise<void> {
		const hub = (await FlowService.getHubFlowById({ id: Number(id) })).flow
		flowViewerFlow = hub
		flowViewer.openDrawer()
	}

	loadHubFlowsWFuse()

	$: {
		if ($workspaceStore && ($userStore || $superadmin)) {
			loadFlows()
		}
	}
</script>

<Drawer bind:this={flowViewer} size="900px">
	<DrawerContent
		title="Hub flow '{flowViewerFlow?.summary ?? ''}'"
		on:close={flowViewer.closeDrawer}
	>
		{#if flowViewerFlow}
			<FlowViewer flow={flowViewerFlow} />
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
		<Tab value="all">All</Tab>
		<Tab value="hub">Hub</Tab>
		<Tab value="personal">{`Personal space (${$userStore?.username})`}</Tab>
		<Tab value="groups">Groups</Tab>
		<Tab value="shared">Shared</Tab>
	</Tabs>

	{#if tab != 'hub'}
		<input type="text" placeholder="Search flows" bind:value={flowFilter} class="search-bar mt-2" />
	{/if}
	<div class="grid grid-cols-1 divide-y">
		{#each tab == 'all' ? ['personal', 'groups', 'shared', 'hub'] : [tab] as sectionTab}
			<div class="shadow p-4 my-2">
				{#if sectionTab == 'personal'}
					<h2>
						<span class="text-lg xl:text-xl">Personal</span>
						<span class="text-sm">
							({`u/${$userStore?.username}`}) <Tooltip>
								All flows owned by you (and visible only to you if you do not explicitly share them)
							</Tooltip></span
						>
					</h2>
				{:else if sectionTab == 'groups'}
					<h2 class="text-lg xl:text-xl">
						Groups <Tooltip>All flows being owned by groups that you are member of</Tooltip>
					</h2>
				{:else if sectionTab == 'shared'}
					<h2 class="text-lg xl:text-xl">
						Shared <Tooltip>All flows visible to you because they have been shared to you</Tooltip>
					</h2>
				{:else if sectionTab == 'hub'}
					<h2 class="text-lg xl:text-xl">
						Approved flows from the WindmillHub <Tooltip>
							All approved Flow from the <a href="https://hub.windmill.dev">WindmillHub</a>.
							Approved flows have been potentially contributed by the community but reviewed and
							selected carefully by the Windmill team.
						</Tooltip>
					</h2>
					<input
						type="text"
						placeholder="Search hub flows"
						bind:value={hubFilter}
						class="search-bar mt-2"
					/>
					<div class="relative mt-2">
						{#if loading.hub}
							<Skeleton
								loading={loading.hub}
								layout={[0.4, [3], 0.4, [3], 0.4, [3], 0.4, [3], 0.4, [3], 0.4, [3], 0.4, [3]]}
							/>
						{:else if hubFlows != undefined}
							<TableCustom>
								<tr slot="header-row">
									<th>Apps</th>
									<th>Summary</th>
									<th />
								</tr>
								<tbody slot="body">
									{#each filteredHubFlows ?? [] as { summary, apps, flow_id }}
										<tr>
											<td class="font-black">{apps.join(', ')}</td>
											<td
												><button class="text-left" on:click={() => viewFlow(flow_id)}
													>{summary}</button
												></td
											>
											<td class="whitespace-nowrap"
												><button class="text-blue-500" on:click={() => viewFlow(flow_id)}
													>view flow</button
												>
												|
												<a target="_blank" href={`https://hub.windmill.dev/flows/${flow_id}`}
													>hub's page
												</a>
												| <a class="font-bold" href={`/flows/add?hub=${flow_id}`}>fork</a>
											</td>
										</tr>
									{/each}
								</tbody>
							</TableCustom>
						{:else}
							<span class="mt-2 text-sm text-red-400">
								Hub not reachable. If your environment is air gapped, contact sales@windmill.dev to
								setup a local mirror.
							</span>
						{/if}
					</div>
				{/if}
				{#each groupedFlows.filter((x) => tabFromPath(x[0]) == sectionTab) as [section, flows]}
					{#if sectionTab != 'personal'}
						<div class="font-bold text-gray-700 mt-2 mb-2">
							{section}
							{#if section == 'g/all'}
								<Tooltip
									>'g/all' is the namespace for the group all. Every user is a member of all.
									Everything in this namespace is visible by all users. At the opposite, 'u/myuser'
									are private user namespaces.</Tooltip
								>
							{/if}
						</div>
					{/if}
					{#if loading.general}
						<div class="grid gap-4 sm:grid-cols-1 md:grid-cols-2 xl:grid-cols-3 mt-2">
							{#each new Array(3) as _}
								<Skeleton layout={[[7.6]]} />
							{/each}
						</div>
					{:else if flows.length == 0 && sectionTab == 'personal'}
						<p class="text-xs text-gray-600 italic mt-2">No flows yet</p>
					{:else}
						<div class="grid md:grid-cols-2 gap-4 sm:grid-cols-1 xl:grid-cols-3 mt-2">
							{#each flows as { summary, path, extra_perms, canWrite }}
								<a
									class="border border-gray-400 p-4 rounded-sm shadow-sm space-y-2 hover:border-blue-600 text-gray-800 flex flex-col justify-between"
									href="/flows/get/{path}"
								>
									<div class="px-6 overflow-auto ">
										<div class="font-semibold text-gray-700">
											{!summary || summary.length == 0 ? path : summary}
										</div>
										<p class="text-gray-700 text-xs">
											<a class="text-gray-700 text-xs" href="/flows/get/{path}">{path} </a>
										</p>
									</div>
									<div class="flex flex-row pl-6 pr-2 mt-2">
										<div class="mr-3 w-full">
											<SharedBadge {canWrite} extraPerms={extra_perms} />
										</div>
										<div class="flex flex-row-reverse w-full place space-x-1">
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
											{#if canWrite}
												<div>
													<Button
														variant="border"
														size="xs"
														href="/flows/edit/{path}"
														endIcon={{ icon: faEdit }}
													>
														Edit
													</Button>
												</div>
											{:else}
												<div>
													<Button
														variant="border"
														size="xs"
														href="/flows/add?template={path}"
														endIcon={{ icon: faCodeFork }}
													>
														Fork
													</Button>
												</div>
											{/if}

											<div>
												<Button
													variant="border"
													size="xs"
													href="/flows/run/{path}"
													endIcon={{ icon: faPlay }}
												>
													Run
												</Button>
											</div>
										</div>
									</div>
								</a>
							{/each}
						</div>
					{/if}
				{/each}
			</div>
		{/each}
	</div>
</CenteredPage>

<ShareModal
	bind:this={shareModal}
	kind="flow"
	on:change={() => {
		loadFlows()
	}}
/>
