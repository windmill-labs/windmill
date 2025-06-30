import type { GetSettingsResponse, LargeFileStorage } from './gen'
import { emptyString } from './utils'

// Extended type to include GCS support until backend types are regenerated

type s3type = 's3' | 'azure_blob' | 's3_aws_oidc' | 'azure_workload_identity' | 'gcloud_storage'
type s3ResourceSettingsItem = {
	resourceType: s3type
	resourcePath: string | undefined
	publicResource: boolean | undefined
}
export type S3ResourceSettings = s3ResourceSettingsItem & {
	secondaryStorage: [string, s3ResourceSettingsItem][] | undefined
}
export function convertBackendSettingsToFrontendSettings(
	large_file_storage: GetSettingsResponse['large_file_storage']
): S3ResourceSettings {
	let settings: Partial<S3ResourceSettings> =
		convertBackendSettingsToFrontendSettingsItem(large_file_storage)
	settings.secondaryStorage = Object.entries(large_file_storage?.secondary_storage ?? {}).map(
		([key, value]) => [key, convertBackendSettingsToFrontendSettingsItem(value)]
	)

	return settings as S3ResourceSettings
}

export function convertBackendSettingsToFrontendSettingsItem(
	large_file_storage: GetSettingsResponse['large_file_storage']
): s3ResourceSettingsItem {
	if (large_file_storage?.type === 'S3Storage') {
		return {
			resourceType: 's3',
			resourcePath: large_file_storage?.s3_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource
		}
	} else if (large_file_storage?.type === 'AzureBlobStorage') {
		return {
			resourceType: 'azure_blob',
			resourcePath: large_file_storage?.azure_blob_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource
		}
	} else if (large_file_storage?.type === 'AzureWorkloadIdentity') {
		return {
			resourceType: 'azure_workload_identity',
			resourcePath: large_file_storage?.azure_blob_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource
		}
	} else if (large_file_storage?.type === 'S3AwsOidc') {
		return {
			resourceType: 's3_aws_oidc',
			resourcePath: large_file_storage?.s3_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource
		}
	} else if (large_file_storage?.type === 'GoogleCloudStorage') {
		const gcsStorage = large_file_storage
		return {
			resourceType: 'gcloud_storage',
			resourcePath: gcsStorage?.gcs_resource_path?.replace('$res:', ''),
			publicResource: gcsStorage?.public_resource
		}
	} else {
		return {
			resourceType: 's3',
			resourcePath: undefined,
			publicResource: undefined
		}
	}
}

export function convertFrontendToBackendSetting(
	s3ResourceSettings: S3ResourceSettings
): LargeFileStorage | undefined {
	let settings = convertFrontendToBackendettingsItem(s3ResourceSettings)
	if (settings) {
		settings.secondary_storage = Object.fromEntries(
			(s3ResourceSettings.secondaryStorage ?? [])
				.map(([key, value]) => [key, convertFrontendToBackendettingsItem(value)])
				.filter(([, value]) => value !== undefined)
		)
	}
	return settings
}
export function convertFrontendToBackendettingsItem(
	s3ResourceSettings: s3ResourceSettingsItem
): LargeFileStorage | undefined {
	if (!emptyString(s3ResourceSettings.resourcePath)) {
		let resourcePathWithPrefix = `$res:${s3ResourceSettings.resourcePath}`
		let params: any = {
			public_resource: s3ResourceSettings.publicResource
		}
		if (s3ResourceSettings.resourceType === 'azure_blob') {
			let typ: LargeFileStorage['type'] = 'AzureBlobStorage'
			params['type'] = typ
			params['azure_blob_resource_path'] = resourcePathWithPrefix
		} else if (s3ResourceSettings.resourceType === 'azure_workload_identity') {
			let typ: LargeFileStorage['type'] = 'AzureWorkloadIdentity'
			params['type'] = typ
			params['azure_blob_resource_path'] = resourcePathWithPrefix
		} else if (s3ResourceSettings.resourceType === 's3_aws_oidc') {
			let typ: LargeFileStorage['type'] = 'S3AwsOidc'
			params['type'] = typ
			params['s3_resource_path'] = resourcePathWithPrefix
		} else if (s3ResourceSettings.resourceType === 'gcloud_storage') {
			let typ: LargeFileStorage['type'] = 'GoogleCloudStorage'
			params['type'] = typ
			params['gcs_resource_path'] = resourcePathWithPrefix
		} else {
			let typ: LargeFileStorage['type'] = 'S3Storage'
			params['type'] = typ
			params['s3_resource_path'] = resourcePathWithPrefix
		}
		return params
	}
}
