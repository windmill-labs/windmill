import { base } from '$lib/base'
import type { RemoteDeployConnection, RemoteDeployTarget } from '$lib/gen'
import { randomSecret } from '$lib/utils/uuid'

/**
 * The workspace argument that aims a generated-client call at `workspace`'s deploy target on
 * another instance. The client fills `{workspace}` with `encodeURI`, which keeps the slashes, so
 * `/w/{workspace}/flows/create` reaches `/w/<workspace>/remote_deploy/proxy/<key>/flows/create`,
 * which the backend forwards to the remote workspace with the caller's token for it.
 */
export function remoteDeployWorkspace(
	workspace: string,
	connection: RemoteDeployConnection
): string {
	return `${workspace}/remote_deploy/proxy/${connection.proxy_key}`
}

export function remoteDeployLabel(target: RemoteDeployTarget): string {
	return `${target.workspace_id} on ${remoteHost(target.base_url)}`
}

export function remoteHost(url: string): string {
	try {
		return new URL(url).host
	} catch {
		return url
	}
}

// Connecting by signing in on the remote: the drawer opens the remote's authorize page, which mints
// a token and sends it back in the fragment of `CALLBACK_PATH`; that page stores it through
// `connect` and tells the drawer on `CHANNEL`.

/** The path the remote's authorize page only ever sends a token to. */
export const CALLBACK_PATH = '/remote_deploy/callback'
export const CHANNEL = 'windmill-remote-deploy'
const PENDING_KEY = 'remote-deploy-connect'
const PENDING_TTL_MS = 15 * 60_000

type PendingConnect = { state: string; workspace: string; returnTo: string; at: number }

export type RemoteDeployEvent =
	| { type: 'connected'; workspace: string }
	| { type: 'failed'; workspace: string; error: string }

/**
 * The remote authorize URL for connecting `workspace`. Records a `state` for the callback to
 * check: without it, any page could send this instance a token of its choosing to store as the
 * user's, and their deploys would land on the remote as someone else. `localStorage` rather than
 * `sessionStorage`, because the callback runs in the window the remote sends back to, which is
 * not always this one.
 */
export function remoteDeployAuthorizeUrl(
	target: RemoteDeployTarget,
	workspace: string,
	returnTo: string
): string {
	const pending: PendingConnect = { state: randomSecret(16), workspace, returnTo, at: Date.now() }
	localStorage.setItem(PENDING_KEY, JSON.stringify(pending))
	const params = new URLSearchParams({
		workspace: target.workspace_id,
		callback: `${window.location.origin}${base}${CALLBACK_PATH}`,
		state: pending.state
	})
	return `${target.base_url}/user/remote_deploy_authorize?${params}`
}

/** The connect `state` names, used once: a replayed callback finds nothing to match. */
export function takePendingConnect(state: string): PendingConnect | undefined {
	let pending: PendingConnect | undefined
	try {
		pending = JSON.parse(localStorage.getItem(PENDING_KEY) ?? 'null') ?? undefined
	} catch {}
	if (!pending || pending.state !== state || Date.now() - pending.at > PENDING_TTL_MS) {
		return undefined
	}
	localStorage.removeItem(PENDING_KEY)
	return pending
}
