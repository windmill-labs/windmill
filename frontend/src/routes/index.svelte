<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import JobDetail from '$lib/components/jobs/JobDetail.svelte'
	import {
		FlowService,
		Job,
		JobService,
		Script,
		ScriptService,
		type Flow,
		type OpenFlow
	} from '$lib/gen'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import {
		Alert,
		Button,
		Drawer,
		DrawerContent,
		Skeleton,
		Tab,
		Tabs,
		ToggleButton,
		ToggleButtonGroup
	} from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import CreateActionsFlow from '$lib/components/flows/CreateActionsFlow.svelte'
	import CreateActionsScript from '$lib/components/scripts/CreateActionsScript.svelte'
	import { canWrite, getScriptByPath, sendUserToast } from '$lib/utils'
	import type { HubItem } from '$lib/components/flows/pickers/model'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import Icon from 'svelte-awesome'
	import {
		faBuilding,
		faCodeFork,
		faGlobe,
		faScroll,
		faWind
	} from '@fortawesome/free-solid-svg-icons'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import PickHubFlow from './PickHubFlow.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import ScriptBox from '$lib/components/ScriptBox.svelte'
	import FlowBox from '$lib/components/FlowBox.svelte'
	import type uFuzzy from '@leeoniya/ufuzzy'

	let jobs: Job[] = []

	type Tab = 'hubscripts' | 'hubflows' | 'workspace'
	type ScriptW = Script & { canWrite: boolean; marked?: string }
	type FlowW = Flow & { canWrite: boolean; marked?: string }
	let scripts: ScriptW[] = []
	let flows: FlowW[] = []
	let filteredItems: ((ScriptW & { type: 'script' }) | (FlowW & { type: 'flow' }))[] = []

	let itemKind: 'script' | 'flow' | 'all' = 'all'

	let tab: Tab = 'workspace'
	let filter: string = ''
	let shareModalScripts: ShareModal
	let shareModalFlows: ShareModal

	let loading = true
	let flowViewer: Drawer
	let flowViewerFlow: { flow?: OpenFlow & { id?: number } } | undefined

	let codeViewer: Drawer
	let codeViewerContent: string = ''
	let codeViewerLanguage: 'deno' | 'python3' | 'go' | 'bash' = 'deno'
	let codeViewerObj: HubItem | undefined = undefined

	async function loadScripts(): Promise<void> {
		scripts = (await ScriptService.listScripts({ workspace: $workspaceStore!, perPage: 300 })).map(
			(x: Script) => {
				return {
					canWrite:
						canWrite(x.path, x.extra_perms, $userStore) && x.workspace_id == $workspaceStore,
					...x
				}
			}
		)
		loading = false
	}

	async function viewCode(obj: HubItem) {
		const { content, language } = await getScriptByPath(obj.path)
		codeViewerContent = content
		codeViewerLanguage = language
		codeViewerObj = obj
		codeViewer.openDrawer()
	}

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
		const hub = await FlowService.getHubFlowById({ id: obj.flow_id })
		flowViewerFlow = hub
		flowViewer.openDrawer()
	}

	$: owners = Array.from(
		new Set(filteredItems?.map((x) => x.path.split('/').slice(0, 2).join('/')) ?? [])
	).sort()

	let combinedItems: (
		| (ScriptW & { type: 'script'; time: number })
		| (FlowW & { type: 'flow'; time: number })
	)[] = []

	$: combinedItems = [
		...flows.map((x) => ({ ...x, type: 'flow' as 'flow', time: new Date(x.edited_at).getTime() })),
		...scripts.map((x) => ({
			...x,
			type: 'script' as 'script',
			time: new Date(x.created_at).getTime()
		}))
	].sort((a, b) => (a.starred != b.starred ? (a.starred ? -1 : 1) : a.time - b.time > 0 ? -1 : 1))

	$: preFilteredItems =
		ownerFilter != undefined
			? combinedItems.filter(
					(x) => x.path.startsWith(ownerFilter ?? '') && (x.type == itemKind || itemKind == 'all')
			  )
			: combinedItems.filter((x) => x.type == itemKind || itemKind == 'all')

	let ownerFilter: string | undefined = undefined

	async function loadJobs() {
		jobs = await JobService.listJobs({
			workspace: $workspaceStore!,
			success: true,
			createdBy: $userStore?.username,
			jobKinds: 'flow,script'
		})
		loading = false
	}

	$: {
		if (($userStore || $superadmin) && $workspaceStore) {
			loadScripts()
			loadFlows()
			loadJobs()
		}
	}
	const cmp = new Intl.Collator('en').compare
	const opts: uFuzzy.Options = {
		sort: (info, haystack, needle) => {
			let {
				idx,
				chars,
				terms,
				interLft2,
				interLft1,
				//	interRgt2,
				//	interRgt1,
				start,
				intraIns,
				interIns
			} = info

			return idx
				.map((v, i) => i)
				.sort(
					(ia, ib) =>
						// most contig chars matched
						chars[ib] - chars[ia] ||
						// least char intra-fuzz (most contiguous)
						intraIns[ia] - intraIns[ib] ||
						// most prefix bounds, boosted by full term matches
						terms[ib] +
							interLft2[ib] +
							0.5 * interLft1[ib] -
							(terms[ia] + interLft2[ia] + 0.5 * interLft1[ia]) ||
						// highest density of match (least span)
						//	span[ia] - span[ib] ||
						// highest density of match (least term inter-fuzz)
						interIns[ia] - interIns[ib] ||
						// earliest start of match
						start[ia] - start[ib] ||
						// alphabetic
						cmp(haystack[idx[ia]], haystack[idx[ib]]) +
							(preFilteredItems[idx[ib]].starred ? 100 : 0) -
							(preFilteredItems[idx[ia]].starred ? 100 : 0)
				)
		}
	}
</script>

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => (x.summary ? x.summary + ' (' + x.path + ')' : x.path)}
	{opts}
/>

<ShareModal
	bind:this={shareModalScripts}
	kind="script"
	on:change={() => {
		loadScripts()
	}}
/>

<ShareModal
	bind:this={shareModalFlows}
	kind="flow"
	on:change={() => {
		loadFlows()
	}}
/>

<Drawer bind:this={codeViewer} size="900px">
	<DrawerContent title={codeViewerObj?.summary ?? ''} on:close={codeViewer.closeDrawer}>
		<div slot="submission" class="flex flex-row gap-2 pr-2"
			><Button
				href="https://hub.windmill.dev/scripts/{codeViewerObj?.app ?? ''}/{codeViewerObj?.ask_id ??
					0}"
				startIcon={{ icon: faGlobe }}
				variant="border">View on the Hub</Button
			><Button
				href="/scripts/add?hub={encodeURIComponent(codeViewerObj?.path ?? '')}"
				startIcon={{ icon: faCodeFork }}>Fork</Button
			></div
		>

		<HighlightCode language={codeViewerLanguage} code={codeViewerContent} />
	</DrawerContent>
</Drawer>

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
	{#if $workspaceStore == 'demo'}
		<div class="my-4" />
		<Alert title="Demo workspace">The demo workspace shared in which all users get invited.</Alert>
	{:else if $workspaceStore == 'starter'}
		<div class="my-4" />

		<Alert title="Stater workspace"
			>The starter workspace has all its elements (variables, resources, scripts, flows) shared
			across all other workspaces. Useful to seed workspace with common elements within your
			organization.</Alert
		>
	{/if}
	<PageHeader title="Home">
		<div class="flex flex-row gap-8">
			<CreateActionsScript />
			<CreateActionsFlow />
		</div>
	</PageHeader>

	<div class="my-6" />
	<Tabs bind:selected={tab}>
		<Tab size="xl" value="workspace"><Icon data={faBuilding} class="mr-2" />Workspace</Tab>
		<Tab value="hubscripts"><Icon data={faGlobe} class="mr-2" />Hub Scripts</Tab>
		<Tab value="hubflows"><Icon data={faGlobe} class="mr-2" />Hub Flows</Tab>
	</Tabs>
	<div class="my-2" />
	<div class="flex flex-col gap-y-16">
		<div class="max-h-screen h-full flex flex-col">
			{#if tab == 'workspace'}
				<div class="w-12/12 pb-2 flex flex-row my-1 gap-1">
					<input
						type="text"
						autofocus
						placeholder="Search Scripts, Flows & Apps"
						bind:value={filter}
						class="text-2xl grow"
					/>
				</div>

				<div class="max-w-min">
					<ToggleButtonGroup bind:selected={itemKind}>
						<ToggleButton light position="left" value="all" size="xs">All</ToggleButton>
						<ToggleButton light position="center" value="script" size="xs"
							><Icon data={faScroll} class="mr-1" />Scripts</ToggleButton
						>
						<ToggleButton light position="right" value="flow" size="xs"
							><Icon data={faWind} class="mr-1" />Flows</ToggleButton
						>
					</ToggleButtonGroup>
				</div>
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

				<div class="grid grid-cols-1 gap-4 lg:grid-cols-2 mt-2 w-full">
					{#if !loading}
						{#each filter != '' ? filteredItems : preFilteredItems as item (item.type + item.path)}
							{#if item.type == 'script'}
								<ScriptBox
									starred={item.starred}
									marked={item.marked}
									on:change={loadScripts}
									script={item}
									shareModal={shareModalScripts}
								/>
							{:else if item.type == 'flow'}
								<FlowBox
									starred={item.starred ?? false}
									marked={item.marked}
									on:change={loadFlows}
									flow={item}
									shareModal={shareModalFlows}
								/>
							{/if}
						{/each}
					{:else}
						{#each Array(10).fill(0) as sk}
							<Skeleton layout={[[4]]} />
						{/each}
					{/if}
				</div>
			{:else if tab == 'hubscripts'}
				<PickHubScript on:pick={(e) => viewCode(e.detail)} />
			{:else if tab == 'hubflows'}
				<PickHubFlow on:pick={(e) => viewFlow(e.detail)} />
			{/if}
		</div>
		<div>
			<h2 class="border-b mb-4 py-2">
				<span class="text-black-gradient">Runs</span>
			</h2>

			<div class="grid grid-cols-1 gap-4 my-4">
				<Skeleton {loading} layout={[[6], 1, [6], 1, [6]]} />
				{#each jobs.splice(0, 3) as job}
					<JobDetail {job} />
				{/each}
				<a
					href="/runs"
					class="text-sm font-extrabold text-gray-700 hover:underline inline-flex items-center"
				>
					All runs
					<svg
						class="w-4 h-4 ml-2"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
						xmlns="http://www.w3.org/2000/svg"
						><path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M17 8l4 4m0 0l-4 4m4-4H3"
						/>
					</svg>
				</a>
			</div>
		</div>
	</div>
</CenteredPage>
