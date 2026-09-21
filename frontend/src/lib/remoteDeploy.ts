import type { RemoteDeployTarget } from '$lib/gen'

/**
 * The workspace argument that aims a generated-client call at `workspace`'s deploy target on
 * another instance. The client fills `{workspace}` with `encodeURI`, which keeps the slashes, so
 * `/w/{workspace}/flows/create` reaches `/w/<workspace>/remote_deploy/proxy/flows/create`, which
 * the backend forwards to the remote workspace with the caller's token for it.
 */
export function remoteDeployWorkspace(workspace: string): string {
	return `${workspace}/remote_deploy/proxy`
}

export function remoteDeployLabel(target: RemoteDeployTarget): string {
	let host = target.base_url
	try {
		host = new URL(target.base_url).host
	} catch {}
	return `${target.workspace_id} on ${host}`
}
