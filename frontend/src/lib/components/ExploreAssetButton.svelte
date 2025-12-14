<script module lang="ts">
	export function assetCanBeExplored(
		asset: Asset,
		_resourceMetadata?: { resource_type?: string }
	): boolean {
		return (
			asset.kind === 'ducklake' ||
			asset.kind === 'datatable' ||
			asset.kind === 's3object' ||
			(asset.kind === 'resource' && isDbType(_resourceMetadata?.resource_type))
		)
	}
</script>

<script lang="ts">
	import { isDbType } from '$lib/components/dbTypes'
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
		buttonVariant = 'default',
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
	unifiedSize={'md'}
	variant={buttonVariant}
	wrapperClasses={className}
	iconOnly={noText}
	{btnClasses}
	on:click={async () => {
		if (asset.kind === 'resource' && isDbType(_resourceMetadata?.resource_type)) {
			let resourcePath = asset.path.split('/').slice(0, 3).join('/')
			let specificTable = asset.path.split('/')[3] as string | undefined
			dbManagerDrawer?.openDrawer({
				type: 'database',
				resourceType: _resourceMetadata.resource_type,
				resourcePath,
				specificTable
			})
		} else if (asset.kind === 's3object' && isS3Uri(assetUri)) {
			s3FilePicker?.open(assetUri)
		} else if (asset.kind === 'ducklake') {
			let ducklake = asset.path.split('/')[0]
			let specificTable = asset.path.split('/')[1] as string | undefined
			dbManagerDrawer?.openDrawer({ type: 'ducklake', ducklake, specificTable })
		} else if (asset.kind === 'datatable') {
			let datatable = asset.path.split('/')[0]
			let specificTable = asset.path.split('/')[1] as string | undefined
			dbManagerDrawer?.openDrawer({
				type: 'database',
				resourceType: 'postgresql',
				resourcePath: `datatable://${datatable}`,
				specificTable
			})
		}
		onClick?.()
	}}
	endIcon={asset.kind === 's3object'
		? { icon: File }
		: asset.kind === 'resource' || asset.kind === 'datatable'
			? { icon: Database }
			: asset.kind === 'ducklake'
				? { icon: DucklakeIcon }
				: undefined}
>
	{#if asset.kind === 's3object'}
		<span class:hidden={noText}>Explore</span>
	{:else if asset.kind === 'resource' || asset.kind === 'ducklake' || asset.kind === 'datatable'}
		<span class:hidden={noText}>Manage</span>
	{/if}
</Button>
