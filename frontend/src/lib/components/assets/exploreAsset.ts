import { isDbType } from '$lib/components/dbTypes'
import { formatAsset, type Asset } from '$lib/components/assets/lib'
import { VolumeService } from '$lib/gen'
import { globalDbManagerDrawer, globalS3FilePickerExplorer } from '$lib/stores'
import { isS3Uri } from '$lib/utils'

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

// An S3FilePicker instance, typed loosely as `globalS3FilePickerExplorer` is: its instance type
// resolves differently in .ts and .svelte contexts.
type S3Picker = any

/** Opens an asset in its drawer: the database manager for a data table, a ducklake or a database
 * resource, the file browser for object storage and volumes. `s3FilePicker` is for a context with
 * its own select/upload flow; otherwise the logged layout's read-only explorer is used. */
export async function exploreAsset(
	asset: Asset,
	opts: { workspace?: string; resourceType?: string; s3FilePicker?: S3Picker }
): Promise<void> {
	const ws = opts.workspace
	const dbManagerDrawer = globalDbManagerDrawer.val
	const s3FilePicker = opts.s3FilePicker ?? globalS3FilePickerExplorer.val
	const assetUri = formatAsset(asset)
	if (asset.kind === 'resource' && isDbType(opts.resourceType)) {
		let [resourcePath, specificTable] = asset.path.split('?table=')
		dbManagerDrawer?.openDrawer(
			{ type: 'database', resourceType: opts.resourceType, resourcePath, specificTable },
			ws
		)
	} else if (asset.kind === 'resource' && isObjectStorageResourceType(opts.resourceType)) {
		s3FilePicker?.open(undefined, { s3ResourcePath: asset.path, workspace: ws })
	} else if (asset.kind === 's3object' && isS3Uri(assetUri)) {
		s3FilePicker?.open(assetUri, { workspace: ws })
	} else if (asset.kind === 'volume') {
		const storage = (await VolumeService.getVolumeStorage({ workspace: ws! })) ?? undefined
		s3FilePicker?.open({ s3: `volumes/${ws}/${asset.path}/`, storage }, { workspace: ws })
	} else if (asset.kind === 'ducklake' || asset.kind === 'datatable') {
		const [name, tablePart] = asset.path.split('/')
		const specificTableSplit = tablePart?.split('.')
		const [specificSchema, specificTable] =
			specificTableSplit?.length === 2
				? [specificTableSplit[0], specificTableSplit[1]]
				: [undefined, specificTableSplit?.[0]]
		dbManagerDrawer?.openDrawer(
			asset.kind === 'ducklake'
				? { type: 'ducklake', ducklake: name, specificSchema, specificTable }
				: {
						type: 'database',
						resourceType: 'postgresql',
						resourcePath: `datatable://${name}`,
						specificTable,
						specificSchema
					},
			ws
		)
	}
}
