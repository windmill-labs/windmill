<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Tab, Tabs } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { Boxes, DollarSign } from 'lucide-svelte'
	import S3Icon from '$lib/components/icons/S3Icon.svelte'
	import ResourceListPage from '$lib/components/assets/ResourceListPage.svelte'
	import VariablesListPage from '$lib/components/assets/VariablesListPage.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import S3ObjectsListPage from '$lib/components/assets/S3ObjectsListPage.svelte'
	import AssetsUsageDropdown from '$lib/components/assets/AssetsUsageDropdown.svelte'

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
	let assetsUsageDropdown: AssetsUsageDropdown | undefined = $state()
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
				onOpenUsages={(usages) => assetsUsageDropdown?.open({ usages })}
			/>
		{:else if selectedTab === 'variables'}
			<VariablesListPage
				bind:this={variablesListPage}
				onOpenUsages={(usages) => assetsUsageDropdown?.open({ usages })}
			/>
		{:else if selectedTab === 's3objects'}
			<S3ObjectsListPage />
		{/if}
	</CenteredPage>
{/if}

<AssetsUsageDropdown bind:this={assetsUsageDropdown} />
