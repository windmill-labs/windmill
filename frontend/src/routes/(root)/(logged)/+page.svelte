<script lang="ts">
	import { AppService, FlowService, Script, type OpenFlow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import CreateActionsFlow from '$lib/components/flows/CreateActionsFlow.svelte'
	import CreateActionsScript from '$lib/components/scripts/CreateActionsScript.svelte'
	import { getScriptByPath } from '$lib/scripts'
	import type { HubItem } from '$lib/components/flows/pickers/model'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import PickHubFlow from '$lib/components/flows/pickers/PickHubFlow.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Building, GitFork, Globe2 } from 'lucide-svelte'

	import ItemsList from '$lib/components/home/ItemsList.svelte'
	import CreateActionsApp from '$lib/components/flows/CreateActionsApp.svelte'
	import PickHubApp from '$lib/components/flows/pickers/PickHubApp.svelte'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import { writable } from 'svelte/store'
	import type { EditorBreakpoint } from '$lib/components/apps/types'
	import { HOME_SHOW_HUB, HOME_SHOW_CREATE_FLOW, HOME_SHOW_CREATE_APP } from '$lib/consts'
	import { setQuery } from '$lib/navigation'
	import { page } from '$app/stores'

	type Tab = 'hub' | 'workspace'

	let tab: Tab =
		window.location.hash == '#workspace' || window.location.hash == '#hub'
			? (window.location.hash?.replace('#', '') as Tab)
			: 'workspace'

	let subtab: 'flow' | 'script' | 'app' = 'script'

	let filter: string = ''

	let flowViewer: Drawer
	let flowViewerFlow: { flow?: OpenFlow & { id?: number } } | undefined

	let appViewer: Drawer
	let appViewerApp: { app?: any & { id?: number } } | undefined

	let codeViewer: Drawer
	let codeViewerContent: string = ''
	let codeViewerLanguage: Script.language = 'deno' as Script.language
	let codeViewerObj: HubItem | undefined = undefined

	const breakpoint = writable<EditorBreakpoint>('lg')

	async function viewCode(obj: HubItem) {
		const { content, language } = await getScriptByPath(obj.path)
		codeViewerContent = content
		codeViewerLanguage = language
		codeViewerObj = obj
		codeViewer.openDrawer?.()
	}

	async function viewFlow(obj: { flow_id: number }): Promise<void> {
		const hub = await FlowService.getHubFlowById({ id: obj.flow_id })
		delete hub['comments']
		flowViewerFlow = hub
		flowViewer.openDrawer?.()
	}

	async function viewApp(obj: { app_id: number }): Promise<void> {
		const hub = await AppService.getHubAppById({ id: obj.app_id })
		delete hub['comments']
		appViewerApp = hub
		appViewer.openDrawer?.()
	}
</script>

<Drawer bind:this={codeViewer} size="900px">
	<DrawerContent title={codeViewerObj?.summary ?? ''} on:close={codeViewer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button
				href="https://hub.windmill.dev/scripts/{codeViewerObj?.app ?? ''}/{codeViewerObj?.ask_id ??
					0}"
				variant="contained"
				color="light"
				size="xs"
				target="_blank"
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>
			<Button
				href="/scripts/add?hub={encodeURIComponent(codeViewerObj?.path ?? '')}"
				startIcon={{ icon: GitFork }}
				color="dark"
				size="xs"
			>
				Fork
			</Button>
		</svelte:fragment>

		<HighlightCode language={codeViewerLanguage} code={codeViewerContent} />
	</DrawerContent>
</Drawer>

<Drawer bind:this={flowViewer} size="1200px">
	<DrawerContent title="Hub flow" on:close={flowViewer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button
				href="https://hub.windmill.dev/flows/{flowViewerFlow?.flow?.id}"
				variant="contained"
				color="light"
				size="xs"
				target="_blank"
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>

			<Button
				href="/flows/add?hub={flowViewerFlow?.flow?.id}"
				startIcon={{ icon: GitFork }}
				color="dark"
				size="xs"
			>
				Fork
			</Button>
		</svelte:fragment>

		{#if flowViewerFlow?.flow}
			<FlowViewer flow={flowViewerFlow.flow} />
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={appViewer} size="1200px">
	<DrawerContent title="Hub app" on:close={appViewer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button
				href="https://hub.windmill.dev/apps/{appViewerApp?.app?.id}"
				variant="contained"
				color="light"
				size="xs"
				target="_blank"
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>

			<Button
				href="/apps/add?hub={appViewerApp?.app?.id}"
				startIcon={{ icon: GitFork }}
				color="dark"
				size="xs"
			>
				Fork
			</Button>
		</svelte:fragment>

		{#if appViewerApp?.app}
			<div class="p-4">
				<AppPreview
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
				/>
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<div>
	<div class="max-w-7xl mx-auto px-4 sm:px-6 md:px-8 h-fit-content">
		{#if $workspaceStore == 'admins'}
			<div class="my-4" />

			<Alert title="Admins workspace">
				The Admins workspace is for admins only and contains scripts whose purpose is to manage your
				Windmill instance, such as keeping resource types up to date.
			</Alert>
		{/if}
		<PageHeader title="Home">
			<div class="flex flex-row gap-4 flex-wrap justify-end items-center">
				{#if !$userStore?.operator}
					<span class="text-sm text-secondary">Create a</span>
					<CreateActionsScript />
					{#if HOME_SHOW_CREATE_FLOW}<CreateActionsFlow />{/if}
					{#if HOME_SHOW_CREATE_APP}<CreateActionsApp />{/if}
				{/if}
			</div>
		</PageHeader>

		{#if !$userStore?.operator}
			<div class="w-full overflow-auto scrollbar-hidden">
				<Tabs values={['hub', 'workspace']} hashNavigation bind:selected={tab}>
					<Tab size="md" value="workspace">
						<div class="flex gap-2 items-center my-1">
							<Building size={18} />
							Workspace
						</div>
					</Tab>
					{#if HOME_SHOW_HUB}
						<Tab size="md" value="hub">
							<div class="flex gap-2 items-center my-1">
								<Globe2 size={18} />
								Hub
							</div>
						</Tab>
					{/if}
				</Tabs>
			</div>
		{/if}
		<div class="my-2" />
		<div class="flex flex-col gap-y-16">
			<div class="flex flex-col">
				{#if tab == 'hub'}
					<Tabs
						bind:selected={subtab}
						on:selected={() => {
							setQuery($page.url, 'kind', subtab, window.location.hash)
						}}
					>
						<Tab size="md" value="script">
							<div class="flex gap-2 items-center my-1"> Scripts </div>
						</Tab>
						<Tab size="md" value="flow">
							<div class="flex gap-2 items-center my-1"> Flows </div>
						</Tab>
						<Tab size="md" value="app">
							<div class="flex gap-2 items-center my-1"> Apps </div>
						</Tab>
					</Tabs>
					<div class="my-2" />

					{#if subtab == 'script'}
						<PickHubScript syncQuery bind:filter on:pick={(e) => viewCode(e.detail)}>
							<Button
								target="_blank"
								href="https://hub.windmill.dev"
								variant="border"
								color="light"
							>
								Go to Hub
							</Button>
						</PickHubScript>
					{:else if subtab == 'flow'}
						<PickHubFlow syncQuery bind:filter on:pick={(e) => viewFlow(e.detail)}>
							<Button target="_blank" href="https://hub.windmill.dev" variant="border" color="light"
								>Go to Hub
							</Button>
						</PickHubFlow>
					{:else if subtab == 'app'}
						<PickHubApp syncQuery bind:filter on:pick={(e) => viewApp(e.detail)}>
							<Button target="_blank" href="https://hub.windmill.dev" variant="border" color="light"
								>Go to Hub</Button
							>
						</PickHubApp>
					{/if}
				{/if}
			</div>
		</div>
	</div>
</div>

{#if tab == 'workspace'}
	<ItemsList bind:filter bind:subtab />
{/if}
