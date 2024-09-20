import { ScriptService, type Script } from '$lib/gen'

export async function deleteScript(hash: string, workspace: string) {
	return await ScriptService.deleteScriptByHash({ workspace: workspace, hash })
}

export async function archiveScript(hash: string, workspace: string) {
	return await ScriptService.archiveScriptByHash({ workspace: workspace, hash })
}

export async function unarchiveScript(hash: string, workspace: string) {
	const r = await ScriptService.getScriptByHash({ workspace: workspace, hash })
	const ns = await ScriptService.createScript({
		workspace: workspace,
		requestBody: {
			...r,
			parent_hash: hash,
			lock: r.lock
		}
	})
	return ns
}

export async function getScriptDeploymentStatus(hash: string, workspace: string): Promise<any> {
	return await ScriptService.getScriptDeploymentStatus({
		workspace: workspace,
		hash: hash
	})
}

export async function loadScript(hash: string, workspace: string): Promise<any> {
	let script: Script
	try {
		script = await ScriptService.getScriptByHash({ workspace: workspace, hash })
	} catch {
		script = await ScriptService.getScriptByPath({ workspace: workspace, path: hash })
		hash = script.hash
	}
	return { script, hash }
}

export function curlCommand(async: boolean, args: any, $page: any, workspace: string, script: any) {
	return `curl -H 'Content-Type: application/json' -H "Authorization: Bearer $TOKEN" -X POST -d '${JSON.stringify(
		args
	)}' ${$page.url.protocol}//${$page.url.hostname}/api/w/${workspace}/jobs/run${
		async ? '' : '_wait_result'
	}/p/${script?.path}`
}
