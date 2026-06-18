<script lang="ts">
	import { AppService, FlowService, type OpenFlow, type Script } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer, DrawerContent } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import FlowIcon from '$lib/components/home/FlowIcon.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import HomeCreateActions from '$lib/components/home/HomeCreateActions.svelte'
	import HomeHero from '$lib/components/home/HomeHero.svelte'
	import FilterSearchbar, {
		useUrlSyncedFilterInstance
	} from '$lib/components/FilterSearchbar.svelte'
	import { buildHomeFilterSchema, type LatestItem } from '$lib/components/home/homeFilter'
	import { getScriptByPath } from '$lib/scripts'
	import type { HubItem } from '$lib/components/flows/pickers/model'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import PickHubFlow from '$lib/components/flows/pickers/PickHubFlow.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import HomeConnectDrawer from '$lib/components/home/HomeConnectDrawer.svelte'
	import {
		Building,
		ExternalLink,
		GitFork,
		Globe2,
		Loader2,
		Code2,
		LayoutDashboard,
		PlugZap,
		SearchCode
	} from 'lucide-svelte'
	import { hubBaseUrlStore } from '$lib/stores'
	import { base } from '$lib/base'

	import ItemsList from '$lib/components/home/ItemsList.svelte'
	import PickHubApp from '$lib/components/flows/pickers/PickHubApp.svelte'
	import { writable } from 'svelte/store'
	import type { EditorBreakpoint } from '$lib/components/apps/types'
	import { HOME_SHOW_HUB } from '$lib/consts'
	import { setQuery } from '$lib/navigation'
	import { page } from '$app/state'
	import { goto, replaceState } from '$app/navigation'
	import ForkWorkspaceBanner from '$lib/components/ForkWorkspaceBanner.svelte'
	import WorkspaceDraftsBanner from '$lib/components/WorkspaceDraftsBanner.svelte'
	import WorkspaceTutorials from '$lib/components/WorkspaceTutorials.svelte'
	import { getContext, onMount, setContext, untrack } from 'svelte'
	import { tutorialsToDo } from '$lib/stores'
	import { ignoredTutorials } from '$lib/components/tutorials/ignoredTutorials'
	import TutorialBanner from '$lib/components/home/TutorialBanner.svelte'
	import NoDirectDeployAlert from '$lib/components/NoDirectDeployAlert.svelte'

	type Tab = 'hub' | 'workspace'

	let tab: Tab = $state(
		window.location.hash == '#workspace' || window.location.hash == '#hub'
			? (window.location.hash?.replace('#', '') as Tab)
			: 'workspace'
	)

	function selectTab(t: Tab) {
		tab = t
		if (typeof window !== 'undefined') window.location.hash = t
	}

	// Unified item-kind toggle (All / Scripts / Flows / Apps). Drives workspace
	// filtering and, for the Hub tab, which picker is shown ('all' → scripts).
	let itemKind: 'all' | 'script' | 'flow' | 'app' = $state(
		(page.url.searchParams.get('kind') as 'all' | 'script' | 'flow' | 'app') ?? 'all'
	)
	let hubKind = $derived(itemKind === 'all' ? 'script' : itemKind)

	// Meta surfaced by ItemsList (owners/labels for the filter schema, latest for
	// the hero). Empty until the workspace list has loaded.
	let homeMeta = $state<{
		owners: string[]
		labels: string[]
		loading: boolean
		latest: LatestItem[]
	}>({ owners: [], labels: [], loading: true, latest: [] })

	let filterUserFoldersType: 'only f/*' | 'u/username and f/*' | undefined = $derived(
		$userStore?.is_super_admin && $userStore.username.includes('@')
			? 'only f/*'
			: $userStore?.is_admin || $userStore?.is_super_admin
				? 'u/username and f/*'
				: undefined
	)

	let homeFilterSchema = $derived(
		buildHomeFilterSchema({
			owners: homeMeta.owners,
			labels: homeMeta.labels,
			showUserFoldersFilter: filterUserFoldersType !== undefined,
			userFoldersLabel:
				filterUserFoldersType === 'only f/*' ? 'Only f/*' : `Only u/${$userStore?.username} and f/*`
		})
	)
	let filters = useUrlSyncedFilterInstance(untrack(() => homeFilterSchema))

	const getFilter = () => filters.val._default_ ?? ''
	const setFilter = (v: string) => (filters.val._default_ = v === '' ? undefined : v)

	let presets = $derived([
		...homeMeta.owners.map((o) => ({ name: o, value: `path:\\ ${o}` })),
		...homeMeta.labels.map((l) => ({ name: l, value: `label:\\ ${l}` }))
	])

	let flowViewer: Drawer | undefined = $state(undefined)
	let flowViewerFlow: { flow?: OpenFlow & { id?: number } } | undefined = $state(undefined)

	let appViewer: Drawer | undefined = $state(undefined)
	let appViewerApp: { app?: any & { id?: number } } | undefined = $state(undefined)

	let codeViewer: Drawer | undefined = $state(undefined)
	let codeViewerContent: string = $state('')
	let codeViewerLanguage: Script['language'] = $state('deno')
	let codeViewerObj: HubItem | undefined = $state(undefined)

	const breakpoint = writable<EditorBreakpoint>('lg')

	async function viewCode(obj: HubItem) {
		codeViewerContent = ''
		codeViewerObj = undefined
		getScriptByPath(obj.path).then(({ content, language }) => {
			codeViewerContent = content
			codeViewerLanguage = language
			codeViewerObj = obj
		})

		codeViewer?.openDrawer?.()
	}

	async function viewFlow(obj: { flow_id: number }): Promise<void> {
		flowViewerFlow = undefined
		FlowService.getHubFlowById({ id: obj.flow_id }).then((hub) => {
			delete hub['comments']
			flowViewerFlow = hub
		})
		flowViewer?.openDrawer?.()
	}

	async function viewApp(obj: { app_id: number }): Promise<void> {
		appViewerApp = undefined
		AppService.getHubAppById({ id: obj.app_id }).then((hub) => {
			delete hub['comments']
			appViewerApp = hub
		})
		appViewer?.openDrawer?.()
	}

	let workspaceTutorials: WorkspaceTutorials | undefined = $state(undefined)
	let homeConnectDrawer: HomeConnectDrawer | undefined = $state(undefined)

	// Provide workspaceTutorials to child components via a reactive wrapper
	let workspaceTutorialsContext = $derived(workspaceTutorials)
	setContext('workspaceTutorials', {
		get value() {
			return workspaceTutorialsContext
		}
	})

	let showCreateButtons = $state(false)

	const openSearchWithPrefilledText: (t?: string) => void = getContext(
		'openSearchWithPrefilledText'
	)

	onMount(() => {
		// Check if there's a tutorial parameter in the URL
		const tutorialParam = page.url.searchParams.get('tutorial')
		if (tutorialParam === 'workspace-onboarding') {
			// Small delay to ensure page is fully loaded
			setTimeout(() => {
				workspaceTutorials?.runTutorialById('workspace-onboarding')
			}, 500)
		} else if (tutorialParam === 'workspace-onboarding-operator') {
			// Small delay to ensure page is fully loaded
			setTimeout(() => {
				workspaceTutorials?.runTutorialById('workspace-onboarding-operator')
			}, 500)
		} else if (!$ignoredTutorials.includes(8) && $tutorialsToDo.includes(8)) {
			// Check if user hasn't completed or ignored the workspace onboarding tutorial
			// Small delay to ensure page is fully loaded
			setTimeout(() => {
				workspaceTutorials?.runTutorialById('workspace-onboarding')
			}, 500)
		}
	})
</script>

<Drawer bind:this={codeViewer} size="900px">
	<DrawerContent title={codeViewerObj?.summary ?? ''} on:close={codeViewer.closeDrawer}>
		{#snippet actions()}
			<Button
				href="{$hubBaseUrlStore}/scripts/{codeViewerObj?.app ?? ''}/{codeViewerObj?.ask_id ?? 0}"
				variant="contained"
				color="light"
				size="xs"
				target="_blank"
				disabled={codeViewerObj == undefined}
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>
			<Button
				href="{base}/scripts/add?hub={encodeURIComponent(codeViewerObj?.path ?? '')}"
				startIcon={{ icon: GitFork }}
				variant="accent"
				size="xs"
				disabled={codeViewerObj == undefined}
			>
				Fork
			</Button>
		{/snippet}
		{#if codeViewerObj != undefined && codeViewerLanguage != undefined}
			<HighlightCode language={codeViewerLanguage} code={codeViewerContent} />
		{:else}
			<div class="p-2">
				<Loader2 class="animate-spin" />
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={flowViewer} size="1200px">
	<DrawerContent title="Hub flow" on:close={flowViewer.closeDrawer}>
		{#snippet actions()}
			<Button
				href="{$hubBaseUrlStore}/flows/{flowViewerFlow?.flow?.id}"
				variant="contained"
				color="light"
				size="xs"
				target="_blank"
				disabled={flowViewerFlow == undefined}
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>

			<Button
				href="{base}/flows/add?hub={flowViewerFlow?.flow?.id}"
				startIcon={{ icon: GitFork }}
				variant="accent"
				size="xs"
				disabled={flowViewerFlow == undefined}
			>
				Fork
			</Button>
		{/snippet}

		{#if flowViewerFlow?.flow}
			{#await import('$lib/components/FlowViewer.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default flow={flowViewerFlow.flow} />
			{/await}
		{:else}
			<div class="p-2">
				<Loader2 class="animate-spin" />
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={appViewer} size="1200px">
	<DrawerContent title="Hub app" on:close={appViewer.closeDrawer}>
		{#snippet actions()}
			<Button
				href="{$hubBaseUrlStore}/apps/{appViewerApp?.app?.id}"
				variant="contained"
				color="light"
				size="xs"
				target="_blank"
				disabled={appViewerApp == undefined}
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>

			<Button
				href="{base}/apps/add?hub={appViewerApp?.app?.id}"
				startIcon={{ icon: GitFork }}
				variant="accent"
				disabled={appViewerApp == undefined}
				size="xs"
			>
				Fork
			</Button>
		{/snippet}

		{#if appViewerApp?.app}
			<div class="p-4">
				{#await import('$lib/components/apps/editor/AppPreview.svelte')}
					<Loader2 class="animate-spin" />
				{:then Module}
					<Module.default
						app={appViewerApp?.app?.value}
						appPath="''"
						{breakpoint}
						policy={{}}
						workspace="hub"
						isEditor={false}
						context={{
							username: $userStore?.username ?? 'anonymous',
							email: $userStore?.email ?? 'anonymous',
							groups: $userStore?.groups ?? []
						}}
						summary={appViewerApp?.app.summary ?? ''}
						noBackend
						replaceStateFn={(path) => replaceState(path, page.state)}
						gotoFn={(path, opt) => goto(path, opt)}
					/>
				{/await}
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<div
	class="flex flex-col w-full h-full overflow-y-auto items-center"
	style="scrollbar-gutter: stable both-edges;"
>
	<ForkWorkspaceBanner />
	<WorkspaceDraftsBanner />
	<div class="max-w-7xl px-4 sm:px-8 md:px-8 h-fit w-full">
		{#if $workspaceStore == 'admins'}
			<div class="my-4"></div>

			<Alert title="Admins workspace">
				The Admins workspace is for admins only and contains scripts whose purpose is to manage your
				Windmill instance, such as keeping resource types up to date.
			</Alert>
		{/if}
		<PageHeader
			title="Home"
			childrenWrapperDivClasses="flex-1 flex flex-row gap-4 flex-wrap justify-end items-center"
		>
			<Button
				variant="default"
				unifiedSize="md"
				startIcon={{ icon: PlugZap }}
				btnClasses="whitespace-nowrap"
				onClick={() => homeConnectDrawer?.openDrawer?.()}
			>
				CLI / MCP
			</Button>
			{#if !$userStore?.operator && showCreateButtons}
				<HomeCreateActions />
			{/if}
		</PageHeader>

		<TutorialBanner />

		<NoDirectDeployAlert onUpdateCanEditStatus={(v) => (showCreateButtons = v)} />

		{#if tab == 'workspace'}
			<HomeHero latest={homeMeta.latest} />
		{/if}

		<!-- Unified toolbar: Workspace/Hub + kind toggles + filter searchbar -->
		<div class="flex flex-wrap items-center gap-2 w-full pt-3 pb-2">
			{#if !$userStore?.operator && HOME_SHOW_HUB}
				<ToggleButtonGroup selected={tab} onSelected={(v) => selectTab(v as Tab)} noWFull>
					{#snippet children({ item })}
						<ToggleButton value="workspace" label="Workspace" icon={Building} {item} />
						<ToggleButton value="hub" label="Hub" icon={Globe2} {item} />
					{/snippet}
				</ToggleButtonGroup>
			{/if}
			<ToggleButtonGroup
				bind:selected={itemKind}
				onSelected={(v) => setQuery(page.url, 'kind', v, window.location.hash)}
				noWFull
			>
				{#snippet children({ item })}
					<ToggleButton value="all" label="All" {item} />
					<ToggleButton value="script" label="Scripts" icon={Code2} {item} />
					<ToggleButton value="flow" label="Flows" icon={FlowIcon} selectedColor="#14b8a6" {item} />
					<ToggleButton
						value="app"
						label="Apps"
						icon={LayoutDashboard}
						selectedColor="#fb923c"
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
			<div class="grow min-w-[12rem]">
				<FilterSearchbar
					schema={homeFilterSchema}
					bind:value={filters.val}
					{presets}
					placeholder="Search scripts, flows & apps..."
				/>
			</div>
			<Button
				onClick={() => openSearchWithPrefilledText?.('#')}
				variant="default"
				unifiedSize="md"
				endIcon={{ icon: SearchCode }}
				btnClasses="whitespace-nowrap"
			>
				Content
			</Button>
			{#if tab == 'hub'}
				<Button
					startIcon={{ icon: ExternalLink }}
					target="_blank"
					href={$hubBaseUrlStore}
					variant="default"
					unifiedSize="md"
				>
					Hub
				</Button>
			{/if}
		</div>

		{#if tab == 'hub'}
			<div class="flex flex-col gap-y-16">
				<div class="flex flex-col pb-8">
					{#if hubKind == 'script'}
						<PickHubScript
							syncQuery
							bind:filter={getFilter, setFilter}
							on:pick={(e) => viewCode(e.detail)}
						/>
					{:else if hubKind == 'flow'}
						<PickHubFlow
							syncQuery
							bind:filter={getFilter, setFilter}
							on:pick={(e) => viewFlow(e.detail)}
						/>
					{:else if hubKind == 'app'}
						<PickHubApp
							syncQuery
							bind:filter={getFilter, setFilter}
							on:pick={(e) => viewApp(e.detail)}
						/>
					{/if}
				</div>
			</div>
		{/if}
	</div>

	{#if tab == 'workspace'}
		<ItemsList
			filterValue={filters.val}
			{itemKind}
			showEditButtons={showCreateButtons}
			onMeta={(m) => (homeMeta = m)}
		/>
	{/if}
</div>

<WorkspaceTutorials bind:this={workspaceTutorials} />
<HomeConnectDrawer bind:this={homeConnectDrawer} />
