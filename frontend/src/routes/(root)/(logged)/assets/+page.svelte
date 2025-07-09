<script lang="ts">
	import { getAssetUsagePageUri } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { DrawerContent, Tab, Tabs } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import DbManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { type AssetUsageAccessType, type AssetUsageKind } from '$lib/gen'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import {
		assetDisplaysAsInputInFlowGraph,
		assetDisplaysAsOutputInFlowGraph
	} from '$lib/components/graph/renderers/nodes/AssetNode.svelte'
	import { Boxes, DollarSign } from 'lucide-svelte'
	import S3Icon from '$lib/components/icons/S3Icon.svelte'
	import ResourceListPage from '$lib/components/assets/ResourceListPage.svelte'
	import VariablesListPage from '$lib/components/assets/VariablesListPage.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import S3ObjectsListPage from '$lib/components/assets/S3ObjectsListPage.svelte'

	let usagesDrawerData:
		| {
				usages: {
					path: string
					kind: AssetUsageKind
					access_type?: AssetUsageAccessType
				}[]
		  }
		| undefined = $state()
	let s3FilePicker: S3FilePicker | undefined = $state()
	let dbManagerDrawer: DbManagerDrawer | undefined = $state()

	let selectedTab: 'resources' | 'variables' | 's3objects' = $state('resources')

	let resourceListPage: ResourceListPage | undefined = $state()
	let variablesListPage: VariablesListPage | undefined = $state()
	let HeaderButtons = $derived.by(() => {
		switch (selectedTab) {
			case 'resources':
				return resourceListPage?.getHeaderButtons()
			case 'variables':
				return variablesListPage?.getHeaderButtons()
			case 's3objects':
				return undefined
		}
	})
</script>

{#if $userStore?.operator && $workspaceStore && !$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.resources}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CenteredPage>
		<PageHeader
			title="Assets"
			tooltip="Manage your assets and see where they are used in the workspace."
			documentationLink="https://www.windmill.dev/docs/core_concepts/assets"
		>
			<div class="flex flex-row justify-end gap-4 min-h-10">
				{@render HeaderButtons?.()}
			</div>
		</PageHeader>
		<Tabs bind:selected={selectedTab} class="mb-6">
			<Tab size="md" value="resources">
				<div class="flex gap-2 items-center my-1 text-primary">
					<Boxes size={18} />
					Resources
					<Tooltip
						documentationLink="https://www.windmill.dev/docs/core_concepts/resources_and_types"
					>
						Save and permission rich objects (JSON) including credentials obtained through OAuth.
					</Tooltip>
				</div>
			</Tab>
			<Tab size="md" value="variables">
				<div class="flex gap-2 items-center my-1 text-primary">
					<DollarSign size={18} />
					Variables
					<Tooltip
						documentationLink="https://www.windmill.dev/docs/https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
					>
						Save and permission strings to be reused in Scripts and Flows.
					</Tooltip>
				</div>
			</Tab>
			<Tab size="md" value="s3objects">
				<div class="flex gap-2 items-center my-1 text-primary">
					<S3Icon />
					S3 Objects
					<Tooltip>
						Windmill auto-detects when you reference an S3 object in your scripts and flows.
					</Tooltip>
				</div>
			</Tab>
		</Tabs>
		{#if selectedTab === 'resources'}
			<ResourceListPage
				bind:this={resourceListPage}
				onOpenUsages={(usages) => (usagesDrawerData = { usages })}
			/>
		{:else if selectedTab === 'variables'}
			<VariablesListPage bind:this={variablesListPage} />
		{:else if selectedTab === 's3objects'}
			<S3ObjectsListPage />
		{/if}
	</CenteredPage>
{/if}

<Drawer
	open={usagesDrawerData !== undefined}
	size="900px"
	on:close={() => (usagesDrawerData = undefined)}
>
	<DrawerContent title="Asset occurrences" on:close={() => (usagesDrawerData = undefined)}>
		<ul class="flex flex-col border rounded-md divide-y">
			{#each usagesDrawerData?.usages ?? [] as u}
				<li>
					<a
						href={getAssetUsagePageUri(u)}
						aria-label={`${u.kind}/${u.path}`}
						class="text-sm text-primary flex items-center py-3 px-4 gap-3 hover:bg-surface-hover cursor-pointer"
					>
						<RowIcon kind={u.kind} />
						<div class="flex flex-col justify-center flex-1">
							<span class="font-semibold">{u.path}</span>
							<span class="text-xs text-tertiary">{u.kind}</span>
						</div>
						<div class="flex gap-2">
							{#if assetDisplaysAsInputInFlowGraph(u)}
								<div class="text-xs border text-tertiary max-w-fit p-1 rounded-md">Read</div>
							{/if}
							{#if assetDisplaysAsOutputInFlowGraph(u)}
								<div class="text-xs border text-tertiary max-w-fit p-1 rounded-md">Write</div>
							{/if}
						</div>
					</a>
				</li>
			{/each}
		</ul>
	</DrawerContent>
</Drawer>

<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
<DbManagerDrawer bind:this={dbManagerDrawer} />
