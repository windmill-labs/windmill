<script module lang="ts">
	export function assetCanBeExplored(
		asset: Asset,
		_resourceMetadata?: { resource_type?: string }
	): boolean {
		return (
			asset.kind === 'ducklake' ||
			asset.kind === 's3object' ||
			(asset.kind === 'resource' && isDbType(_resourceMetadata?.resource_type))
		)
	}
</script>

<script lang="ts">
	import { isDbType } from '$lib/components/apps/components/display/dbtable/utils'
	import { formatAsset, type Asset } from '$lib/components/assets/lib'
	import { Button, ButtonType } from '$lib/components/common'
	import DbManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { userStore } from '$lib/stores'
	import { isS3Uri } from '$lib/utils'
	import { Database, File } from 'lucide-svelte'
	import DucklakeIcon from './icons/DucklakeIcon.svelte'

	const {
		asset,
		_resourceMetadata,
		s3FilePicker,
		dbManagerDrawer,
		onClick,
		class: className = '',
		noText = false,
		buttonVariant = 'border',
		btnClasses = '',
		disabled = false
	}: {
		asset: Asset
		_resourceMetadata?: { resource_type?: string }
		s3FilePicker?: S3FilePicker
		dbManagerDrawer?: DbManagerDrawer
		onClick?: () => void
		class?: string
		noText?: boolean
		buttonVariant?: ButtonType.Variant
		btnClasses?: string
		disabled?: boolean
	} = $props()
	const assetUri = $derived(formatAsset(asset))
</script>

<Button
	disabled={$userStore?.operator || disabled}
	size="xs"
	variant={buttonVariant}
	spacingSize="xs2"
	wrapperClasses={className}
	{btnClasses}
	on:click={async () => {
		if (asset.kind === 'resource' && isDbType(_resourceMetadata?.resource_type)) {
			dbManagerDrawer?.openDrawer({
				type: 'database',
				resourceType: _resourceMetadata.resource_type,
				resourcePath: asset.path
			})
		} else if (asset.kind === 's3object' && isS3Uri(assetUri)) {
			s3FilePicker?.open(assetUri)
		} else if (asset.kind === 'ducklake') {
			dbManagerDrawer?.openDrawer({ type: 'ducklake', ducklake: asset.path })
		}
		onClick?.()
	}}
>
	{#if asset.kind === 's3object'}
		<span class:hidden={noText}>Explore</span> <File size={18} />
	{:else if asset.kind === 'resource'}
		<span class:hidden={noText}>Manage</span> <Database size={18} />
	{:else if asset.kind === 'ducklake'}
		<span class:hidden={noText}>Manage</span> <DucklakeIcon size={18} />
	{/if}
</Button>
