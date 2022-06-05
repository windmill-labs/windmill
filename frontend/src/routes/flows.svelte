<script lang="ts">
	import Fuse from 'fuse.js'
	import { FlowService } from '../gen'
	import type { Flow } from '../gen'

	import { sendUserToast, groupBy, canWrite } from '../utils'
	import Icon from 'svelte-awesome'
	import {
		faArchive,
		faCalendarAlt,
		faEdit,
		faEye,
		faList,
		faPlay,
		faPlus,
		faShare
	} from '@fortawesome/free-solid-svg-icons'

	import Dropdown from './components/Dropdown.svelte'
	import PageHeader from './components/PageHeader.svelte'
	import Tooltip from './components/Tooltip.svelte'
	import ShareModal from './components/ShareModal.svelte'
	import SharedBadge from './components/SharedBadge.svelte'
	import { superadmin, usernameStore, userStore, workspaceStore } from '../stores'
	import CenteredPage from './components/CenteredPage.svelte'
	import Tabs from './components/Tabs.svelte'

	type Tab = 'all' | 'personal' | 'groups' | 'shared'
	type Section = [string, FlowW[]]
	type FlowW = Flow & { canWrite: boolean; tab: Tab }
	let flows: FlowW[] = []
	let filteredFlows: FlowW[]
	let flowFilter = ''
	let groupedFlows: Section[] = []

	let tab: Tab = 'all'

	let shareModal: ShareModal

	const fuseOptions = {
		includeScore: false,
		keys: ['description', 'path', 'content', 'hash', 'summary']
	}
	const fuse: Fuse<FlowW> = new Fuse(flows, fuseOptions)

	$: filteredFlows =
		flowFilter.length > 0 ? fuse.search(flowFilter).map((value) => value.item) : flows

	$: {
		let defaults: string[] = []

		if (tab == 'all' || tab == 'personal') {
			defaults = defaults.concat(`u/${$usernameStore}`)
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
		if (path_prefix[0] == 'u' && path_prefix[1] == $usernameStore) {
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
		fuse.setCollection(flows)
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

	$: {
		if ($workspaceStore && ($userStore || $superadmin)) {
			loadFlows()
		}
	}
</script>

<CenteredPage>
	<PageHeader title="Flows" tooltip="Flows can compose and chain scripts together">
		<div class="flex flex-row">
			<a class="default-button" href="/flows/add"
				><Icon class="text-white mb-1" data={faPlus} scale={0.9} /> &nbsp; New flow</a
			>
		</div>
	</PageHeader>

	<Tabs
		tabs={[
			['all', 'all'],
			['personal', `personal space (${$usernameStore})`],
			['groups', 'groups'],
			['shared', 'shared']
		]}
		bind:tab
		on:update={loadFlows}
	/>
	<input placeholder="Search flows" bind:value={flowFilter} class="search-bar mt-2" />
	<div class="grid grid-cols-1 divide-y">
		{#each tab == 'all' ? ['personal', 'groups', 'shared'] : [tab] as sectionTab}
			<div class="shadow p-4 my-2">
				{#if sectionTab == 'personal'}
					<h2 class="">
						My personal space ({`u/${$usernameStore}`})
					</h2>
					<p class="italic text-xs text-gray-600 mb-4">
						All flows owned by you (and visible only to you if you do not explicitely share them)
						will be displayed below
					</p>
				{:else if sectionTab == 'groups'}
					<h2 class="">Groups that I am member of</h2>
					<p class="italic text-xs text-gray-600">
						All flows being owned by groups that you are member of will be displayed below
					</p>
				{:else if sectionTab == 'shared'}
					<h2 class="">Shared with me</h2>
					<p class="italic text-xs text-gray-600">
						All flows visible to you because they have been shared to you will be displayed below
					</p>
				{/if}
				{#each groupedFlows.filter((x) => tabFromPath(x[0]) == sectionTab) as [section, flows]}
					{#if sectionTab != 'personal'}
						<h3 class="mt-2 mb-2">
							owner: {section}
							{#if section == 'g/all'}
								<Tooltip class="mx-1"
									>'g/all' is the namespace for the group all. Every user is a member of all.
									Everything in this namespace is visible by all users. At the opposite, 'u/myuser'
									are private user namespaces.</Tooltip
								>
							{/if}
						</h3>
					{/if}
					{#if flows.length == 0}
						<p class="text-xs text-gray-600 font-black">
							No flows for this owner space yet. To create one, click on the top right button.
						</p>
					{:else}
						<div class="grid md:grid-cols-2 gap-4 sm:grid-cols-1 2xl:grid-cols-3">
							{#each flows as { summary, path, extra_perms, canWrite }}
								<div
									class="flex flex-col justify-between flow max-w-lg overflow-visible shadow-sm shadow-blue-100 border border-gray-200 bg-gray-50 py-2"
								>
									<a href="/flows/get/{path}">
										<div class="px-6 overflow-auto ">
											<div class="font-semibold text-gray-700">
												{!summary || summary.length == 0 ? path : summary}
											</div>
											<p class="text-gray-700 text-xs">
												<a class="text-gray-700 text-xs" href="/flows/get/{path}">Path: {path} </a>
											</p>
										</div>
									</a>
									<div class="flex flex-row pl-6 pr-2 mt-2">
										<div class="mr-3 w-full">
											<SharedBadge {canWrite} extraPerms={extra_perms} />
										</div>
										<div class="flex flex-row-reverse w-full place">
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
																shareModal.openModal(path)
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
												<a
													class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
													href="/flows/run/{path}"
												>
													<div class="inline-flex items-center justify-center">
														<Icon data={faPlay} scale={0.5} />
														<span class="pl-1">Run...</span>
													</div>
												</a>
											</div>
										</div>
									</div>
								</div>
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

<style>
	.selected:hover {
		@apply border border-gray-500 rounded-md border-opacity-50;
	}

	.flow:hover {
		@apply border border-gray-600 border-opacity-60;
	}
</style>
