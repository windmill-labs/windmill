<script lang="ts">
	import { AppService, FlowService, IntegrationService, type OpenFlow, type Script } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import FlowIcon from '$lib/components/home/FlowIcon.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import CreateActionsFlow from '$lib/components/flows/CreateActionsFlow.svelte'
	import CreateActionsScript from '$lib/components/scripts/CreateActionsScript.svelte'
	import { getScriptByPath } from '$lib/scripts'
	import type { HubItem } from '$lib/components/flows/pickers/model'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import PickHubFlow from '$lib/components/flows/pickers/PickHubFlow.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import {
		Building,
		ExternalLink,
		GitFork,
		Globe2,
		Loader2,
		Code,
		LayoutDashboard,
		Route,
		Tag,
		Type
	} from 'lucide-svelte'
	import { hubBaseUrlStore } from '$lib/stores'
	import { base } from '$lib/base'

	import ItemsList from '$lib/components/home/ItemsList.svelte'
	import FilterSearchbar, { type FilterSchemaRec } from '$lib/components/FilterSearchbar.svelte'
	import CreateActionsApp from '$lib/components/flows/CreateActionsApp.svelte'
	import PickHubApp from '$lib/components/flows/pickers/PickHubApp.svelte'
	import { writable } from 'svelte/store'
	import type { EditorBreakpoint } from '$lib/components/apps/types'
	import { HOME_SHOW_HUB, HOME_SHOW_CREATE_FLOW, HOME_SHOW_CREATE_APP } from '$lib/consts'
	import { page } from '$app/state'
	import { goto, replaceState } from '$app/navigation'
	import ForkWorkspaceBanner from '$lib/components/ForkWorkspaceBanner.svelte'
	import WorkspaceTutorials from '$lib/components/WorkspaceTutorials.svelte'
	import { onMount, setContext } from 'svelte'
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

	let subtab: 'flow' | 'script' | 'app' = $state('script')

	// Hub filter: load integration names for tag suggestions
	let hubIntegrations: string[] = $state([])

	// Hub filter schema (tag options update when integrations load)
	let hubFilterSchema: FilterSchemaRec = $derived({
		_default_: { type: 'string', hidden: true },
		tag: {
			type: 'oneof',
			label: 'Tag',
			description: 'Filter by integration/app tag',
			icon: Tag,
			allowCustomValue: true,
			options: hubIntegrations.map((name) => ({ value: name, label: name }))
		},
		summary: {
			type: 'string',
			label: 'Summary',
			description: 'Filter by summary text',
			icon: Type
		},
		path: { type: 'string', label: 'Path', description: 'Filter by path', icon: Route },
		kind: {
			type: 'oneof',
			label: 'Kind',
			description: 'Filter by runnable type',
			options: [
				{ value: 'script', label: 'Script' },
				{ value: 'flow', label: 'Flow' },
				{ value: 'app', label: 'App' }
			]
		}
	})

	let hubFilterValue: Record<string, any> = $state({})
	let hubFreeText = $derived((hubFilterValue._default_ as string) ?? '')
	// Merge _default_ free text into summary filter for hub components
	let hubSummaryFilter = $derived(
		[hubFilterValue.summary, hubFreeText].filter(Boolean).join(' ') || undefined
	)
	let hubAppFilter: string | undefined = $state(undefined)

	// Sync hub kind filter ↔ subtab
	$effect(() => {
		const k = hubFilterValue.kind as string | undefined
		if (k === 'script' || k === 'flow' || k === 'app') {
			subtab = k
		}
	})

	// Sync hub tag filter → hubAppFilter (for PickHub* API calls)
	$effect(() => {
		const tag = hubFilterValue.tag as string | undefined
		hubAppFilter = tag ?? undefined
	})

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

	// Provide workspaceTutorials to child components via a reactive wrapper
	let workspaceTutorialsContext = $derived(workspaceTutorials)
	setContext('workspaceTutorials', {
		get value() {
			return workspaceTutorialsContext
		}
	})

	let showCreateButtons = $state(false)

	onMount(async () => {
		// Load hub integrations for tag suggestions
		try {
			hubIntegrations = (await IntegrationService.listHubIntegrations({ kind: 'script' })).map(
				(x) => x.name
			)
		} catch {
			hubIntegrations = []
		}

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
			{#if !$userStore?.operator && showCreateButtons}
				<span class="text-xs font-normal text-primary">Create a</span>
				<CreateActionsScript aiId="create-script-button" aiDescription="Creates a new script" />
				{#if HOME_SHOW_CREATE_FLOW}<CreateActionsFlow />{/if}
				{#if HOME_SHOW_CREATE_APP}<CreateActionsApp />{/if}
			{/if}
		</PageHeader>

		<TutorialBanner />

		<NoDirectDeployAlert onUpdateCanEditStatus={(v) => (showCreateButtons = v)} />

		{#if !$userStore?.operator}
			<div class="w-full overflow-auto scrollbar-hidden pb-2">
				<Tabs values={['hub', 'workspace']} hashNavigation bind:selected={tab}>
					<Tab value="workspace" label="Workspace" icon={Building} />
					{#if HOME_SHOW_HUB}
						<Tab value="hub" label="Hub" icon={Globe2} />
					{/if}
				</Tabs>
			</div>
		{/if}
		{#if tab == 'hub'}
			<div class="flex flex-col gap-y-16">
				<div class="flex flex-col pb-8">
					<div class="w-full flex items-center gap-2">
						<ToggleButtonGroup
							bind:selected={subtab}
							onSelected={(v) => {
								hubFilterValue.kind = v
							}}
							noWFull
						>
							{#snippet children({ item })}
								<ToggleButton value="script" label="Scripts" icon={Code} {item} />
								<ToggleButton
									value="flow"
									label="Flows"
									icon={FlowIcon}
									selectedColor="#14b8a6"
									{item}
								/>
								<ToggleButton
									value="app"
									label="Apps"
									icon={LayoutDashboard}
									selectedColor="#fb923c"
									{item}
								/>
							{/snippet}
						</ToggleButtonGroup>
						<FilterSearchbar
							schema={hubFilterSchema}
							bind:value={hubFilterValue}
							placeholder="Search hub..."
							class="grow"
							presets={[
								{ name: 'Slack', value: 'tag:slack' },
								{ name: 'Gmail', value: 'tag:gmail' },
								{ name: 'GitHub', value: 'tag:github' },
								{ name: 'Notion', value: 'tag:notion' },
								{ name: 'Stripe', value: 'tag:stripe' },
								{ name: 'OpenAI', value: 'tag:openai' },
								{ name: 'HubSpot', value: 'tag:hubspot' },
								{ name: 'PostgreSQL', value: 'tag:postgresql' },
								{ name: 'Google Sheets', value: 'tag:gsheets' },
								{ name: 'Shopify', value: 'tag:shopify' },
								{ name: 'Other...', value: 'tag:' }
							]}
						/>
						<Button
							startIcon={{ icon: ExternalLink }}
							target="_blank"
							href={$hubBaseUrlStore}
							variant="default"
						>
							Hub
						</Button>
					</div>

					{#if subtab == 'script'}
						<PickHubScript
							appFilter={hubAppFilter}
							summaryFilter={hubSummaryFilter}
							pathFilter={hubFilterValue.path}
							hideSearchbar
							on:pick={(e) => viewCode(e.detail)}
						/>
					{:else if subtab == 'flow'}
						<PickHubFlow
							appFilter={hubAppFilter}
							summaryFilter={hubSummaryFilter}
							pathFilter={hubFilterValue.path}
							hideSearchbar
							on:pick={(e) => viewFlow(e.detail)}
						/>
					{:else if subtab == 'app'}
						<PickHubApp
							appFilter={hubAppFilter}
							summaryFilter={hubSummaryFilter}
							pathFilter={hubFilterValue.path}
							hideSearchbar
							on:pick={(e) => viewApp(e.detail)}
						/>
					{/if}
				</div>
			</div>
		{/if}
	</div>

	{#if tab == 'workspace'}
		<ItemsList bind:subtab showEditButtons={showCreateButtons} />
	{/if}
</div>

<WorkspaceTutorials bind:this={workspaceTutorials} />
