import type { GetSettingsResponse, LargeFileStorage } from './gen'
import { emptyString } from './utils'

// Extended type to include GCS support until backend types are regenerated

type S3type = 's3' | 'azure_blob' | 's3_aws_oidc' | 'azure_workload_identity' | 'gcloud_storage'
export type S3ResourceSettingsItem = {
	resourceType: S3type
	resourcePath: string | undefined
	publicResource: boolean | undefined
	advancedPermissions?: {
		pattern: string
		allow: ('read' | 'write' | 'delete' | 'list')[]
	}[]
}
export type S3ResourceSettings = S3ResourceSettingsItem & {
	secondaryStorage: [string, S3ResourceSettingsItem][] | undefined
}
export function convertBackendSettingsToFrontendSettings(
	large_file_storage: GetSettingsResponse['large_file_storage'],
	isEnterprise: boolean
): S3ResourceSettings {
	let settings: Partial<S3ResourceSettings> = convertBackendSettingsToFrontendSettingsItem(
		large_file_storage,
		isEnterprise
	)
	settings.secondaryStorage = Object.entries(large_file_storage?.secondary_storage ?? {}).map(
		([key, value]) => [key, convertBackendSettingsToFrontendSettingsItem(value, isEnterprise)]
	)

	return settings as S3ResourceSettings
}

export function convertBackendSettingsToFrontendSettingsItem(
	large_file_storage: GetSettingsResponse['large_file_storage'],
	isEnterprise: boolean
): S3ResourceSettingsItem {
	let advancedPermissions = large_file_storage?.advanced_permissions
		? large_file_storage.advanced_permissions.map((rule) => ({
				...rule,
				allow: rule.allow
					?.split(',')
					.filter((rule) => !!rule)
					.map((rule) => rule as 'read' | 'write' | 'delete' | 'list')
			}))
		: undefined
	if (large_file_storage?.type === 'S3Storage') {
		return {
			resourceType: 's3',
			resourcePath: large_file_storage?.s3_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource,
			advancedPermissions
		}
	} else if (large_file_storage?.type === 'AzureBlobStorage') {
		return {
			resourceType: 'azure_blob',
			resourcePath: large_file_storage?.azure_blob_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource,
			advancedPermissions
		}
	} else if (large_file_storage?.type === 'AzureWorkloadIdentity') {
		return {
			resourceType: 'azure_workload_identity',
			resourcePath: large_file_storage?.azure_blob_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource,
			advancedPermissions
		}
	} else if (large_file_storage?.type === 'S3AwsOidc') {
		return {
			resourceType: 's3_aws_oidc',
			resourcePath: large_file_storage?.s3_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource,
			advancedPermissions
		}
	} else if (large_file_storage?.type === 'GoogleCloudStorage') {
		return {
			resourceType: 'gcloud_storage',
			resourcePath: large_file_storage?.gcs_resource_path?.replace('$res:', ''),
			publicResource: large_file_storage?.public_resource,
			advancedPermissions
		}
	} else {
		return {
			resourceType: 's3',
			resourcePath: undefined,
			publicResource: undefined,
			advancedPermissions: defaultS3AdvancedPermissions(isEnterprise)
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
	s3ResourceSettings: S3ResourceSettingsItem
): LargeFileStorage | undefined {
	if (!emptyString(s3ResourceSettings.resourcePath)) {
		let resourcePathWithPrefix = `$res:${s3ResourceSettings.resourcePath}`
		let params = {
			public_resource: s3ResourceSettings.publicResource,
			...(s3ResourceSettings.advancedPermissions
				? {
						advanced_permissions: s3ResourceSettings.advancedPermissions.map((rule) => ({
							...rule,
							allow: rule.allow.join(',')
						}))
					}
				: {})
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

export function defaultS3AdvancedPermissions(
	isEnterprise: boolean
): S3ResourceSettingsItem['advancedPermissions'] {
	if (!isEnterprise) return undefined
	return [
		{ pattern: 'windmill_uploads/*', allow: ['read', 'write', 'delete'] },
		{ pattern: 'u/{username}/**/*', allow: ['read', 'write', 'delete', 'list'] },
		{ pattern: 'g/{group}/**/*', allow: ['read', 'write', 'delete', 'list'] },
		{ pattern: 'f/{folder_write}/**/*', allow: ['read', 'write', 'delete', 'list'] },
		{ pattern: 'f/{folder_read}/**/*', allow: ['read', 'list'] },
		{ pattern: '**/*', allow: [] }
	]
}
