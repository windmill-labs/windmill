<script lang="ts">
	import { formatAsset } from '$lib/components/assets/lib'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, DrawerContent } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { Cell, DataTable } from '$lib/components/table'
	import Head from '$lib/components/table/Head.svelte'
	import { AssetService, type ListAssetsResponse } from '$lib/gen'
	import { userStore, workspaceStore, userWorkspaces } from '$lib/stores'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { isS3Uri, pluralize } from '$lib/utils'
	import { File } from 'lucide-svelte'

	let assets = usePromise(() => AssetService.listAssets({ workspace: $workspaceStore ?? '' }))

	let viewOccurences: ListAssetsResponse[number] | undefined = $state()
	let s3FilePicker: S3FilePicker | undefined = $state()
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
					<Cell head></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface">
				{#each assets.value ?? [] as item}
					{@const assetUri = formatAsset(item)}
					<tr>
						<Cell first>{assetUri}</Cell>
						<Cell>
							<a href={`#${assetUri}`} onclick={() => (viewOccurences = item)}>
								{pluralize(item.usages.length, 'occurrence')}
							</a>
						</Cell>
						<Cell>
							{#if item.kind === 's3object'}
								<Button
									size="xs"
									variant="border"
									spacingSize="xs2"
									btnClasses="mt-1 w-24"
									on:click={async () => isS3Uri(assetUri) && s3FilePicker?.open(assetUri)}
								>
									<File size={18} /> View
								</Button>
							{/if}
						</Cell>
					</tr>
				{/each}
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
			<ul class="px-6">
				<li class="text-sm list-disc">{u.usage_kind}/{u.usage_path}</li>
			</ul>
		{/each}
	</DrawerContent>
</Drawer>
<S3FilePicker bind:this={s3FilePicker} readOnlyMode />
