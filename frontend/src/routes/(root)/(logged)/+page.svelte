<script lang="ts">
	import { AppService, FlowService, type OpenFlow, type Script } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import CreateActionsFlow from '$lib/components/flows/CreateActionsFlow.svelte'
	import CreateActionsScript from '$lib/components/scripts/CreateActionsScript.svelte'
	import { getScriptByPath } from '$lib/scripts'
	import type { HubItem } from '$lib/components/flows/pickers/model'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import PickHubFlow from '$lib/components/flows/pickers/PickHubFlow.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Building, ExternalLink, GitFork, Globe2, Loader2 } from 'lucide-svelte'
	import { hubBaseUrlStore } from '$lib/stores'
	import { base } from '$lib/base'

	import ItemsList from '$lib/components/home/ItemsList.svelte'
	import CreateActionsApp from '$lib/components/flows/CreateActionsApp.svelte'
	import PickHubApp from '$lib/components/flows/pickers/PickHubApp.svelte'
	import { writable } from 'svelte/store'
	import type { EditorBreakpoint } from '$lib/components/apps/types'
	import { HOME_SHOW_HUB, HOME_SHOW_CREATE_FLOW, HOME_SHOW_CREATE_APP } from '$lib/consts'
	import { setQuery } from '$lib/navigation'
	import { page } from '$app/stores'
	import { goto, replaceState } from '$app/navigation'
	import AskAiButton from '$lib/components/AskAiButton.svelte'
	import { AIChatService } from '$lib/components/copilot/chat/AIChatManager.svelte'

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
	let codeViewerLanguage: Script['language'] = 'deno'
	let codeViewerObj: HubItem | undefined = undefined

	const breakpoint = writable<EditorBreakpoint>('lg')

	async function viewCode(obj: HubItem) {
		codeViewerContent = ''
		codeViewerObj = undefined
		getScriptByPath(obj.path).then(({ content, language }) => {
			codeViewerContent = content
			codeViewerLanguage = language
			codeViewerObj = obj
		})

		codeViewer.openDrawer?.()
	}

	async function viewFlow(obj: { flow_id: number }): Promise<void> {
		flowViewerFlow = undefined
		FlowService.getHubFlowById({ id: obj.flow_id }).then((hub) => {
			delete hub['comments']
			flowViewerFlow = hub
		})
		flowViewer.openDrawer?.()
	}

	async function viewApp(obj: { app_id: number }): Promise<void> {
		appViewerApp = undefined
		AppService.getHubAppById({ id: obj.app_id }).then((hub) => {
			delete hub['comments']
			appViewerApp = hub
		})
		appViewer.openDrawer?.()
	}
</script>

<Drawer bind:this={codeViewer} size="900px">
	<DrawerContent title={codeViewerObj?.summary ?? ''} on:close={codeViewer.closeDrawer}>
		<svelte:fragment slot="actions">
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
				color="dark"
				size="xs"
				disabled={codeViewerObj == undefined}
			>
				Fork
			</Button>
		</svelte:fragment>
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
		<svelte:fragment slot="actions">
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
				color="dark"
				size="xs"
				disabled={flowViewerFlow == undefined}
			>
				Fork
			</Button>
		</svelte:fragment>

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
		<svelte:fragment slot="actions">
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
				color="dark"
				disabled={appViewerApp == undefined}
				size="xs"
			>
				Fork
			</Button>
		</svelte:fragment>

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
						replaceStateFn={(path) => replaceState(path, $page.state)}
						gotoFn={(path, opt) => goto(path, opt)}
					/>
				{/await}
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<div>
	{#if !AIChatService.open}
		<div class="fixed top-4 right-2 z-50">
			<AskAiButton />
		</div>
	{/if}
	<div class="max-w-7xl mx-auto px-4 sm:px-8 md:px-8 h-fit-content">
		{#if $workspaceStore == 'admins'}
			<div class="my-4"></div>

			<Alert title="Admins workspace">
				The Admins workspace is for admins only and contains scripts whose purpose is to manage your
				Windmill instance, such as keeping resource types up to date.
			</Alert>
		{/if}
		<PageHeader title="Home">
			<div class="flex flex-row gap-4 flex-wrap justify-end items-center">
				{#if !$userStore?.operator}
					<span class="text-sm text-secondary">Create a</span>
					<CreateActionsScript aiId="create-script-button" aiDescription="Creates a new script" />
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
		<div class="my-2"></div>
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
					<div class="my-2"></div>

					{#if subtab == 'script'}
						<PickHubScript syncQuery bind:filter on:pick={(e) => viewCode(e.detail)}>
							<Button
								startIcon={{ icon: ExternalLink }}
								target="_blank"
								href={$hubBaseUrlStore}
								variant="border"
								color="light"
							>
								Hub
							</Button>
						</PickHubScript>
					{:else if subtab == 'flow'}
						<PickHubFlow syncQuery bind:filter on:pick={(e) => viewFlow(e.detail)}>
							<Button
								startIcon={{ icon: ExternalLink }}
								target="_blank"
								href={$hubBaseUrlStore}
								variant="border"
								color="light"
								>Hub
							</Button>
						</PickHubFlow>
					{:else if subtab == 'app'}
						<PickHubApp syncQuery bind:filter on:pick={(e) => viewApp(e.detail)}>
							<Button
								startIcon={{ icon: ExternalLink }}
								target="_blank"
								href={$hubBaseUrlStore}
								variant="border"
								color="light">Hub</Button
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
