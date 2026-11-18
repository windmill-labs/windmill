import type { AppInput, EvalInputV2 } from '../inputType'
import type { App, RichConfigurations } from '../types'
import { collectOneOfFields } from './appUtilsCore'
function filenameExprToRegex(template: string) {
	const filenameEscaped = template.replaceAll('${file.name}', '<file_name>') // replace filename with placeholder
	const escapedTemplate = filenameEscaped
		.slice(1, -1) // remove quotes
		.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') // escape regex special characters
	const regexPattern = escapedTemplate.replaceAll('<file_name>', '[^/]+') // replace filename placeholder with regex pattern
	return `^${regexPattern}$`
}

function defaultIfEmptyString(str: string | undefined, dflt: string): string {
	return str === undefined || str === null || str === '' ? dflt : str!
}

function staticToRegex(str: string) {
	return `^${str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`
}

function checkIfExprIsString(input: string) {
	return /^(['"`])[^'"`]*\1$/g.test(input)
}

function checkIfEvalIsStringWithFilename(input: EvalInputV2) {
	if (input.connections.length > 0) {
		return false
	} else {
		return checkIfExprIsString(input.expr.replaceAll('${file.name}', ''))
	}
}

function removeResourcePrefix(resource: string) {
	return resource.replace(/^\$res:/, '')
}

export function computeWorkspaceS3FileInputPolicy() {
	return {
		allow_workspace_resource: true,
		allowed_resources: [],
		allow_user_resources: false,
		file_key_regex: ''
	}
}

export function computeS3FileInputPolicy(s3Config: any, app: App) {
	const resourceInput = s3Config?.resource as AppInput | undefined
	const pathTemplateInput = s3Config?.pathTemplate as AppInput | undefined

	const allow_workspace_resource =
		!resourceInput || (resourceInput.type === 'static' && !resourceInput.value)
	const allowed_resources: string[] = resourceInput
		? resourceInput.type === 'static'
			? resourceInput.value
				? [removeResourcePrefix(resourceInput.value)]
				: []
			: (collectOneOfFields(
					{
						s3_resource: resourceInput
					},
					app
				).s3_resource?.map((s) => removeResourcePrefix(s)) ?? [])
		: []

	const allow_user_resources =
		(resourceInput?.type === 'evalv2' && resourceInput?.allowUserResources) ?? false
	let file_key_regex = '^.*$'
	if (pathTemplateInput) {
		if (pathTemplateInput.type === 'static') {
			file_key_regex = staticToRegex(pathTemplateInput.value)
		} else if (
			pathTemplateInput.type === 'evalv2' &&
			checkIfEvalIsStringWithFilename(pathTemplateInput)
		) {
			file_key_regex = filenameExprToRegex(pathTemplateInput.expr)
		}
	}

	return {
		allow_workspace_resource,
		allowed_resources,
		allow_user_resources,
		file_key_regex
	}
}

export function isPartialS3Object(
	input: unknown
): input is { s3: string; storage?: string; presigned?: string } {
	return input != undefined && typeof input === 'object' && typeof input['s3'] === 'string'
}

export async function getS3File({
	source,
	storage,
	presigned,
	appPath,
	username,
	workspace,
	token,
	isEditor
}: {
	source: string | undefined
	storage?: string
	presigned?: string
	appPath: string
	username: string | undefined
	workspace: string
	token: string | undefined
	isEditor: boolean
	// Optional; not read here. Editor reads go through the viewer-scoped endpoint
	// and deployed reads through the app-scoped one, independent of the component
	// configuration.
	configuration?: RichConfigurations
}) {
	if (!source) return ''

	// Editor/preview runs execute as the *caller* (Viewer mode), so read their
	// results back as the caller through the viewer-scoped `job_helpers` endpoint —
	// never author-mode — consistent with DisplayResult/ParqetCsvTableRenderer. Only
	// a deployed app view reads on-behalf of the author via the provenance-gated
	// `apps_u` endpoint.
	if (isEditor) {
		const params = new URLSearchParams()
		params.append('file_key', source)
		if (storage) {
			params.append('storage', storage)
		}
		if (token && token != '') {
			params.append('token', token)
		}
		return `/api/w/${workspace}/job_helpers/download_s3_file?${params.toString()}${presigned ? `&${presigned}` : ''}`
	}

	const appPathOrUser = defaultIfEmptyString(appPath, `u/${username ?? 'unknown'}/newapp`)
	const params = new URLSearchParams()
	params.append('s3', source)
	if (storage) {
		params.append('storage', storage)
	}
	if (token && token != '') {
		params.append('token', token)
	}

	return `/api/w/${workspace}/apps_u/download_s3_file/${appPathOrUser}?${params.toString()}${presigned ? `&${presigned}` : ''}`
}

export function computeS3FileViewerPolicy(config: RichConfigurations) {
	if (config.source.type === 'uploadS3' && isPartialS3Object(config.source.value)) {
		return {
			s3_path: config.source.value.s3,
			storage: config.source.value.storage
		}
	} else if (
		config.source.type === 'static' &&
		typeof config.source.value === 'string' &&
		((config.sourceKind?.type === 'static' &&
			config.sourceKind?.value === 's3 (workspace storage)') ||
			config.source.value.startsWith('s3://'))
	) {
		return {
			s3_path: config.source.value.replace('s3://', ''),
			storage: undefined
		}
	} else {
		return undefined
	}
}
