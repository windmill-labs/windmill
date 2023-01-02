<script lang="ts">
	import { FlowService, type OpenFlow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import CreateActionsFlow from '$lib/components/flows/CreateActionsFlow.svelte'
	import CreateActionsScript from '$lib/components/scripts/CreateActionsScript.svelte'
	import { getScriptByPath } from '$lib/utils'
	import type { HubItem } from '$lib/components/flows/pickers/model'
	import { faCodeFork } from '@fortawesome/free-solid-svg-icons'
	import PickHubScript from '$lib/components/flows/pickers/PickHubScript.svelte'
	import PickHubFlow from '$lib/components/flows/pickers/PickHubFlow.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Building, Globe2 } from 'lucide-svelte'

	import ItemsList from '$lib/components/home/ItemsList.svelte'
	import CreateActionsApp from '$lib/components/flows/CreateActionsApp.svelte'

	type Tab = 'hubscripts' | 'hubflows' | 'workspace'

	let tab: Tab = 'workspace'
	let filter: string = ''

	let flowViewer: Drawer
	let flowViewerFlow: { flow?: OpenFlow & { id?: number } } | undefined

	let codeViewer: Drawer
	let codeViewerContent: string = ''
	let codeViewerLanguage: 'deno' | 'python3' | 'go' | 'bash' = 'deno'
	let codeViewerObj: HubItem | undefined = undefined

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
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>
			<Button
				href="/scripts/add?hub={encodeURIComponent(codeViewerObj?.path ?? '')}"
				startIcon={{ icon: faCodeFork }}
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
			>
				<div class="flex gap-2 items-center">
					<Globe2 size={18} />
					View on the Hub
				</div>
			</Button>

			<Button
				href="/flows/add?hub={flowViewerFlow?.flow?.id}"
				startIcon={{ icon: faCodeFork }}
				color="dark"
				size="xs"
			>
				Fork
			</Button>
		</svelte:fragment>

		{#if flowViewerFlow?.flow}
			<div class="p-4">
				<FlowViewer flow={flowViewerFlow.flow} />
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<div>
	<div class="max-w-6xl mx-auto px-4 sm:px-6 md:px-8 h-fit-content">
		{#if $workspaceStore == 'demo'}
			<div class="my-4" />
			<Alert title="Demo workspace">All users get an invitation to this workspace.</Alert>
		{:else if $workspaceStore == 'starter'}
			<div class="my-4" />

			<Alert title="Starter workspace">
				The starter workspace has all its elements (variables, resources, scripts, flows) shared
				across all other workspaces. Useful to seed workspace with common elements within your
				organization.
			</Alert>
		{:else if $workspaceStore == 'admins'}
			<div class="my-4" />

			<Alert title="Admins workspace">
				The Admins workspace is for admins only and contains scripts whose purpose is to manage your
				Windmill instance, such as keeping resource types up to date.
			</Alert>
		{/if}
		<PageHeader title="Home">
			<div class="flex flex-row gap-3 flex-wrap justify-end">
				{#if !$userStore?.operator}
					<CreateActionsScript />
					<CreateActionsFlow />
					<CreateActionsApp />
				{/if}
			</div>
		</PageHeader>

		{#if !$userStore?.operator}
			<Tabs bind:selected={tab}>
				<Tab size="md" value="workspace">
					<div class="flex gap-2 items-center my-1">
						<Building size={18} />
						Workspace
					</div>
				</Tab>
				<Tab size="md" value="hubscripts">
					<div class="flex gap-2 items-center my-1">
						<Globe2 size={18} />
						Hub Scripts
					</div>
				</Tab>
				<Tab size="md" value="hubflows">
					<div class="flex gap-2 items-center my-1">
						<Globe2 size={18} />
						Hub Flows
					</div>
				</Tab>
			</Tabs>
		{/if}
		<div class="my-2" />
		<div class="flex flex-col gap-y-16">
			<div class="flex flex-col">
				{#if tab == 'hubscripts'}
					<PickHubScript bind:filter on:pick={(e) => viewCode(e.detail)} />
				{:else if tab == 'hubflows'}
					<PickHubFlow bind:filter on:pick={(e) => viewFlow(e.detail)} />
				{/if}
			</div>
		</div>
	</div>
</div>

{#if tab == 'workspace'}
	<ItemsList bind:filter />
{/if}
