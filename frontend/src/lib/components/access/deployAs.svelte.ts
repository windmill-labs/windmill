import { resource } from 'runed'
import { GranularAclService } from '$lib/gen'

// Every run-as control on a page asks, so one request per workspace serves them for a minute.
const TTL_MS = 60_000
const asked = new Map<string, { at: number; accounts: Promise<string[]> }>()

function fetchDeployAs(workspace: string): Promise<string[]> {
	const hit = asked.get(workspace)
	if (hit && Date.now() - hit.at < TTL_MS) return hit.accounts
	const accounts = GranularAclService.listDeployAs({ workspace }).then(
		(res) => res.service_accounts.map((a) => `u/${a.username}`),
		() => []
	)
	asked.set(workspace, { at: Date.now(), accounts })
	return accounts
}

export function forgetDeployAs(workspace: string): void {
	asked.delete(workspace)
}

/** The service accounts the current user may deploy as in `workspace`, as `u/<username>`: those
 *  whose deployers list names them, one of their groups, or a folder they write in. Admins and
 *  `wm_deployers` get every account, and may deploy as anyone besides. */
export function useDeployAs(workspace: () => string | undefined) {
	const accounts = resource(workspace, (ws) => (ws ? fetchDeployAs(ws) : Promise.resolve([])))
	return {
		get current(): string[] {
			return accounts.current ?? []
		}
	}
}
