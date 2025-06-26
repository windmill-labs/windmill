<script module lang="ts">
	export function assetCanBeExplored(
		asset: Asset,
		_resourceMetadata?: { resourceType?: string }
	): boolean {
		return (
			asset.kind === 's3object' ||
			(asset.kind === 'resource' && isDbType(_resourceMetadata?.resourceType))
		)
	}
</script>

<script lang="ts">
	import { isDbType } from '$lib/components/apps/components/display/dbtable/utils'
	import { formatAsset, type Asset } from '$lib/components/assets/lib'
	import { Button } from '$lib/components/common'
	import DbManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { isS3Uri } from '$lib/utils'
	import { Database, File } from 'lucide-svelte'

	const {
		asset,
		_resourceMetadata,
		s3FilePicker,
		dbManagerDrawer,
		onClick,
		class: className = '',
		noText = false
	}: {
		asset: Asset
		_resourceMetadata?: { resourceType?: string }
		s3FilePicker?: S3FilePicker
		dbManagerDrawer?: DbManagerDrawer
		onClick?: () => void
		class?: string
		noText?: boolean
	} = $props()
	const assetUri = $derived(formatAsset(asset))
</script>

<Button
	size="xs"
	variant="border"
	spacingSize="xs2"
	wrapperClasses={className}
	on:click={async () => {
		if (asset.kind === 'resource' && isDbType(_resourceMetadata?.resourceType)) {
			dbManagerDrawer?.openDrawer(_resourceMetadata.resourceType, asset.path)
		} else if (asset.kind === 's3object' && isS3Uri(assetUri)) {
			s3FilePicker?.open(assetUri)
		}
		onClick?.()
	}}
>
	{#if asset.kind === 's3object'}
		<span class:hidden={noText}>Explore</span> <File size={18} />
	{:else if asset.kind === 'resource'}
		<span class:hidden={noText}>Manage</span> <Database size={18} />
	{/if}
</Button>
