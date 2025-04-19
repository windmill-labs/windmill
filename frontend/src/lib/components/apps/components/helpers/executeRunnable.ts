import { AppService, type ExecuteComponentData } from '$lib/gen'
import { defaultIfEmptyString } from '$lib/utils'
import type { Runnable } from '../../inputType'
import type { InlineScript } from '../../types'

export async function executeRunnable(
	runnable: Runnable,
	workspace: string,
	version: number | undefined,
	username: string | undefined,
	path: string,
	id: string,
	requestBody: ExecuteComponentData['requestBody'],
	inlineScriptOverride?: InlineScript
) {
	let appPath = defaultIfEmptyString(path, `u/${username ?? 'unknown'}/newapp`)
	if (runnable?.type === 'runnableByName') {
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
	} else if (runnable?.type === 'runnableByPath') {
		const { path, runType } = runnable
		requestBody['path'] = runType !== 'hubscript' ? `${runType}/${path}` : `script/${path}`
	}

	if (version !== undefined) {
		requestBody['version'] = version
	}

	const uuid = await AppService.executeComponent({
		workspace,
		path: appPath,
		requestBody
	})

	return uuid
}
