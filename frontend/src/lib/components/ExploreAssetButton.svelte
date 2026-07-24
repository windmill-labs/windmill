<script module lang="ts">
	const OBJECT_STORAGE_RESOURCE_TYPES = [
		's3',
		'azure_blob',
		's3_aws_oidc',
		'azure_workload_identity',
		'gcloud_storage'
	]

	export function isObjectStorageResourceType(resourceType: string | undefined): boolean {
		return resourceType !== undefined && OBJECT_STORAGE_RESOURCE_TYPES.includes(resourceType)
	}

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
	import { formatAsset, type Asset } from '$lib/components/assets/lib'
	import { Button, ButtonType } from '$lib/components/common'
	import S3FilePicker from '$lib/components/S3FilePicker.svelte'
	import { VolumeService } from '$lib/gen'
	import {
		globalDbManagerDrawer,
		globalS3FilePickerExplorer,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import { isS3Uri } from '$lib/utils'
	import { Database, File, HardDriveIcon } from 'lucide-svelte'
	import DucklakeIcon from './icons/DucklakeIcon.svelte'

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

	let dbManagerDrawer = $derived(globalDbManagerDrawer.val)
	let ws = $derived(workspace ?? $workspaceStore)
	const assetUri = $derived(formatAsset(asset))
	// Contexts with a select/upload flow pass their own picker; everything else
	// (e.g. the resources list) falls back to the global read-only explorer.
	let effectiveS3FilePicker = $derived(s3FilePicker ?? globalS3FilePickerExplorer.val)
	let isStorageResource = $derived(
		asset.kind === 'resource' && isObjectStorageResourceType(_resourceMetadata?.resource_type)
	)
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
			let [resourcePath, specificTable] = asset.path.split('?table=')
			dbManagerDrawer?.openDrawer(
				{
					type: 'database',
					resourceType: _resourceMetadata.resource_type,
					resourcePath,
					specificTable
				},
				ws
			)
		} else if (isStorageResource) {
			effectiveS3FilePicker?.open(undefined, { s3ResourcePath: asset.path })
		} else if (asset.kind === 's3object' && isS3Uri(assetUri)) {
			effectiveS3FilePicker?.open(assetUri)
		} else if (asset.kind === 'volume') {
			const storage = (await VolumeService.getVolumeStorage({ workspace: ws! })) ?? undefined
			effectiveS3FilePicker?.open({ s3: `volumes/${ws}/${asset.path}/`, storage })
		} else if (asset.kind === 'ducklake') {
			let ducklake = asset.path.split('/')[0]
			let specificTableSplit = asset.path.split('/')[1]?.split('.') as string[] | undefined
			let [specificSchema, specificTable] =
				specificTableSplit?.length === 2
					? [specificTableSplit[0], specificTableSplit[1]]
					: [undefined, specificTableSplit?.[0]]
			dbManagerDrawer?.openDrawer({ type: 'ducklake', ducklake, specificSchema, specificTable }, ws)
		} else if (asset.kind === 'datatable') {
			let datatable = asset.path.split('/')[0]
			let specificTableSplit = asset.path.split('/')[1]?.split('.') as string[] | undefined
			let [specificSchema, specificTable] =
				specificTableSplit?.length === 2
					? [specificTableSplit[0], specificTableSplit[1]]
					: [undefined, specificTableSplit?.[0]]
			dbManagerDrawer?.openDrawer(
				{
					type: 'database',
					resourceType: 'postgresql',
					resourcePath: `datatable://${datatable}`,
					specificTable,
					specificSchema
				},
				ws
			)
		}
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
