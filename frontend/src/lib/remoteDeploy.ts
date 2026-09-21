import { OpenAPI, type RemoteDeployTarget } from '$lib/gen'

const PROXY_SUFFIX = '/remote_deploy/proxy'

/** Mirrors `REMOTE_DEPLOY_HEADER` in the backend's `remote_deploy.rs`, which refuses any
 * proxied request without it so that a link cannot spend a stored remote token. */
const REMOTE_DEPLOY_HEADER = 'X-Windmill-Remote-Deploy'

/**
 * The workspace argument that aims a generated-client call at `workspace`'s deploy target on
 * another instance. The client fills `{workspace}` with `encodeURI`, which keeps the slashes, so
 * `/w/{workspace}/flows/create` reaches `/w/<workspace>/remote_deploy/proxy/flows/create`, which
 * the backend forwards to the remote workspace with the caller's token for it.
 */
export function remoteDeployWorkspace(workspace: string): string {
	return `${workspace}${PROXY_SUFFIX}`
}

export function remoteDeployLabel(target: RemoteDeployTarget): string {
	let host = target.base_url
	try {
		host = new URL(target.base_url).host
	} catch {}
	return `${target.workspace_id} on ${host}`
}

// Registered once for the session, chained onto whatever resolver was there, and a no-op for any
// call not addressed to the proxy.
const previousHeaders = OpenAPI.HEADERS
OpenAPI.HEADERS = async (options) => {
	const headers =
		typeof previousHeaders === 'function' ? await previousHeaders(options) : previousHeaders
	const workspace = options.path?.workspace
	return typeof workspace === 'string' && workspace.endsWith(PROXY_SUFFIX)
		? { ...headers, [REMOTE_DEPLOY_HEADER]: '1' }
		: (headers ?? {})
}
