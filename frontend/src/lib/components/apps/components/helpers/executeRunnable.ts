import { AppService, type ExecuteComponentData } from '$lib/gen'
import { defaultIfEmptyString } from '$lib/utils'
import { isRunnableByName, isRunnableByPath, type Runnable } from '../../inputType'
import type { InlineScript } from '../../sharedTypes'

export async function executeRunnable(
	runnable: Runnable,
	workspace: string,
	version: number | undefined,
	username: string | undefined,
	path: string,
	id: string,
	requestBody: ExecuteComponentData['requestBody'],
	inlineScriptOverride?: InlineScript,
	queryParams?: Record<string, any>,
) {
	let appPath = defaultIfEmptyString(path, `u/${username ?? 'unknown'}/newapp`)
	if (isRunnableByName(runnable)) {
		const { inlineScript } = inlineScriptOverride
			? { inlineScript: inlineScriptOverride }
			: runnable

		if (inlineScript) {
			if (inlineScript.id !== undefined) {
				requestBody['id'] = inlineScript.id
			}
			requestBody['raw_code'] = {
				content: inlineScript.id === undefined ? inlineScript.content : '',
				language: inlineScript.language ?? '',
				path: appPath + '/' + id,
				lock: inlineScript.id === undefined ? inlineScript.lock : undefined,
				cache_ttl: inlineScript.cache_ttl
			}
		}
	} else if (isRunnableByPath(runnable)) {
		const { path, runType } = runnable
		requestBody['path'] = runType !== 'hubscript' ? `${runType}/${path}` : `script/${path}`
	}

	if (version !== undefined) {
		requestBody['version'] = version
	}

	if (queryParams && Object.keys(queryParams).length > 0) {
		requestBody['run_query_params'] = queryParams
	}

	const uuid = await AppService.executeComponent({
		workspace,
		path: appPath,
		requestBody
	})

	return uuid
}
