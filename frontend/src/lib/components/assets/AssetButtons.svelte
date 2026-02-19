<script lang="ts">
	import { AlertTriangle, Edit2 } from 'lucide-svelte'
	import { Button } from '../common'
	import ExploreAssetButton, { assetCanBeExplored } from '../ExploreAssetButton.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import type { Asset } from '$lib/gen'
	import Popover from '../meltComponents/Popover.svelte'

	type Props = {
		s3FilePicker?: any | undefined
		dbManagerDrawer?: any | undefined
		resourceEditorDrawer?: ResourceEditorDrawer | undefined
		resourceDataCache: Record<string, string | undefined>
		asset: Asset
		ducklakeNotFound?: boolean
		datatableNotFound?: boolean
		onClick?: () => void
	}
	let {
		s3FilePicker,
		dbManagerDrawer,
		resourceEditorDrawer,
		resourceDataCache,
		asset,
		ducklakeNotFound = false,
		datatableNotFound = false,
		onClick
	}: Props = $props()

	let truncatedPath = asset.path.split('?table=')[0]
	let resourceDataCacheValue = $derived(resourceDataCache[truncatedPath])
</script>

<div class="flex gap-2 items-center">
	{#if asset.kind === 'resource' && resourceDataCacheValue !== undefined}
		<Button
			startIcon={{ icon: Edit2 }}
			variant="default"
			unifiedSize="md"
			iconOnly
			on:click={() => (resourceEditorDrawer?.initEdit(truncatedPath), onClick?.())}
		/>
	{/if}
	{#if (asset.kind === 'resource' && resourceDataCacheValue === undefined) || ducklakeNotFound || datatableNotFound}
		<Popover contentClasses="px-3 py-2">
			<svelte:fragment slot="trigger">
				<Button
					startIcon={{ icon: AlertTriangle }}
					variant="default"
					unifiedSize="md"
					btnClasses="text-red-500"
					iconOnly
				/>
			</svelte:fragment>
			<svelte:fragment slot="content">
				<span class="text-sm">Not found</span>
				{#if ducklakeNotFound}
					<Button wrapperClasses="mt-1" href="/workspace_settings?tab=windmill_lfs">
						Go to Ducklake settings
					</Button>
				{:else if datatableNotFound}
					<Button wrapperClasses="mt-1" href="/workspace_settings?tab=windmill_data_tables">
						Go to Data Table settings
					</Button>
				{:else if asset.kind === 'resource' && resourceDataCacheValue === undefined}
					<Button wrapperClasses="mt-1" href="/resources">Go to Resources</Button>
				{/if}
			</svelte:fragment>
		</Popover>
	{:else if assetCanBeExplored(asset, { resource_type: resourceDataCacheValue })}
		<ExploreAssetButton
			{asset}
			{s3FilePicker}
			{dbManagerDrawer}
			onClick={() => onClick?.()}
			noText
			_resourceMetadata={{ resource_type: resourceDataCacheValue }}
		/>
	{/if}
</div>
