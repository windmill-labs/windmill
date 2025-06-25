<script lang="ts">
	import { formatAsset } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, DrawerContent } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Cell, DataTable } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import { AssetService } from '$lib/gen'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { usePaginated } from '$lib/svelte5Utils.svelte'
	import { pluralize } from '$lib/utils'
	import { RefreshCw } from 'lucide-svelte'

	let assets = usePaginated(async (page) => {
		return {
			items: await AssetService.list({
				workspace: $workspaceStore ?? '',
				page,
				perPage: 50
			})
		}
	})

	let viewOccurences: (typeof assets)['items'][number] | undefined = $state()
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
			tooltip="Track where your assets are used in Windmill."
			documentationLink="https://www.windmill.dev/docs/core_concepts/assets"
		/>
		<DataTable>
			<Head>
				<tr>
					<Cell head first>Asset name</Cell>
					<Cell head></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface">
				{#each assets.items as item}
					{@const assetUri = formatAsset(item)}
					<tr>
						<Cell first>{assetUri}</Cell>
						<Cell>
							<a href={`#${assetUri}`} onclick={() => (viewOccurences = item)}>
								{pluralize(item.usages.length, 'occurrence')}
							</a>
						</Cell>
					</tr>
				{/each}
				<tr class="w-full">
					<td colspan={99} class="p-1">
						<Button
							wrapperClasses="mx-auto"
							size="xs"
							startIcon={{ icon: RefreshCw }}
							color="light"
							on:click={() => assets.loadMore()}
						>
							Load more
						</Button>
					</td>
				</tr>
			</tbody>
		</DataTable>
	</CenteredPage>
{/if}

<Drawer
	open={viewOccurences !== undefined}
	size="900px"
	on:close={() => (viewOccurences = undefined)}
>
	<DrawerContent title="Asset occurrences" on:close={() => (viewOccurences = undefined)}>
		{#each viewOccurences?.usages ?? [] as u}
			<div class="p-4">
				<h3 class="text-lg font-semibold mb-2">{u.usage_kind}</h3>
				<p class="text-sm text-gray-600 mb-2">{u.usage_path}</p>
			</div>
		{/each}
	</DrawerContent>
</Drawer>
