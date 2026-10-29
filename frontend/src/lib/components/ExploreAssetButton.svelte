<script module lang="ts">
	import { isObjectStorageResourceType } from './assets/exploreAsset'

	export function assetCanBeExplored(
		asset: Asset,
		_resourceMetadata?: { resource_type?: string }
	): boolean {
		return (
			asset.kind === 'ducklake' ||
			asset.kind === 'datatable' ||
			asset.kind === 's3object' ||
			asset.kind === 'volume' ||
			(asset.kind === 'resource' &&
				(isDbType(_resourceMetadata?.resource_type) ||
					isObjectStorageResourceType(_resourceMetadata?.resource_type)))
		)
	}
</script>

<script lang="ts">
	import { isDbType } from '$lib/components/dbTypes'
	import { type Asset } from '$lib/components/assets/lib'
	import { Button, ButtonType } from '$lib/components/common'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { exploreAsset } from './assets/exploreAsset'
	import { Database, File, HardDriveIcon } from 'lucide-svelte'
	import DucklakeIcon from './icons/DucklakeIcon.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { useRoleCapabilities } from '$lib/operatorWriteRights'

	const operatingWorkspace = useOperatingWorkspace()
	const roleCaps = useRoleCapabilities()

	const {
		asset,
		_resourceMetadata,
		s3FilePicker,
		onClick,
		class: className = '',
		noText = false,
		buttonVariant = 'default',
		btnClasses = '',
		disabled = false,
		workspace = undefined
	}: {
		asset: Asset
		_resourceMetadata?: { resource_type?: string }
		s3FilePicker?: S3FilePicker
		onClick?: () => void
		class?: string
		noText?: boolean
		buttonVariant?: ButtonType.Variant
		btnClasses?: string
		disabled?: boolean
		/** Workspace the explored asset lives in; defaults to the nav workspace. */
		workspace?: string
	} = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)
	let isStorageResource = $derived(
		asset.kind === 'resource' && isObjectStorageResourceType(_resourceMetadata?.resource_type)
	)
</script>

<Button
	disabled={!$roleCaps.code || disabled}
	unifiedSize={'md'}
	variant={buttonVariant}
	wrapperClasses={className}
	iconOnly={noText}
	{btnClasses}
	on:click={async () => {
		await exploreAsset(asset, {
			workspace: ws,
			resourceType: _resourceMetadata?.resource_type,
			s3FilePicker
		})
		onClick?.()
	}}
	endIcon={asset.kind === 's3object' || isStorageResource
		? { icon: File }
		: asset.kind === 'resource' || asset.kind === 'datatable'
			? { icon: Database }
			: asset.kind === 'ducklake'
				? { icon: DucklakeIcon }
				: asset.kind === 'volume'
					? { icon: HardDriveIcon }
					: undefined}
>
	{#if asset.kind === 's3object' || asset.kind === 'volume' || isStorageResource}
		<span class:hidden={noText}>Explore</span>
	{:else if asset.kind === 'resource' || asset.kind === 'ducklake' || asset.kind === 'datatable'}
		<span class:hidden={noText}>Manage</span>
	{/if}
</Button>
