<script lang="ts">
	import { AlertTriangle, Edit2 } from 'lucide-svelte'
	import { Button } from '../common'
	import { Tooltip } from '../meltComponents'
	import ExploreAssetButton, { assetCanBeExplored } from '../ExploreAssetButton.svelte'
	import S3FilePicker from '../S3FilePicker.svelte'
	import DbManagerDrawer from '../DBManagerDrawer.svelte'
	import ResourceEditorDrawer from '../ResourceEditorDrawer.svelte'
	import type { Asset } from '$lib/gen'

	type Props = {
		s3FilePicker?: S3FilePicker | undefined
		dbManagerDrawer?: DbManagerDrawer | undefined
		resourceEditorDrawer?: ResourceEditorDrawer | undefined
		resourceDataCache: Record<string, string | undefined>
		asset: Asset
		onClick?: () => void
	}
	let {
		s3FilePicker,
		dbManagerDrawer,
		resourceEditorDrawer,
		resourceDataCache,
		asset,
		onClick
	}: Props = $props()
</script>

<div class="flex gap-2 items-center">
	{#if asset.kind === 'resource' && resourceDataCache[asset.path] !== undefined}
		<Button
			startIcon={{ icon: Edit2 }}
			size="xs"
			variant="border"
			spacingSize="xs2"
			iconOnly
			on:click={() => (resourceEditorDrawer?.initEdit(asset.path), onClick?.())}
		/>
	{/if}
	{#if asset.kind === 'resource' && resourceDataCache[asset.path] === undefined}
		<Tooltip class="mr-2.5">
			<AlertTriangle size={16} class="text-orange-600 dark:text-orange-500" />
			<svelte:fragment slot="text">Could not find resource</svelte:fragment>
		</Tooltip>
	{/if}
	{#if assetCanBeExplored(asset, { resource_type: resourceDataCache[asset.path] })}
		<ExploreAssetButton
			{asset}
			{s3FilePicker}
			{dbManagerDrawer}
			onClick={() => onClick?.()}
			noText
			_resourceMetadata={{ resource_type: resourceDataCache[asset.path] }}
		/>
	{/if}
</div>
