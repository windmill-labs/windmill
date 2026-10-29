import { resource } from 'runed'
import { SvelteMap } from 'svelte/reactivity'
import { FolderService, GranularAclService, UserService, WorkspaceService } from '$lib/gen'
import { setGrant } from './access'
import { forgetDeployAs } from './deployAs.svelte'
import { sendUserToast } from '$lib/toast'

export type ServiceAccount = {
	username: string
	email: string
	/** Folders shared with the account by name. */
	folders: string[]
	/** Items shared with it one by one; known only for a pending account. */
	items?: string[]
	/** Chosen in a picker, created when the item deploys. */
	pending?: boolean
}

export type Grant = { kind: 'folder' | 'resource' | 'variable'; path: string }

/** A service account picked before it exists: the deploy creates it, then gives it the rest. */
export type PendingServiceAccount = {
	username: string
	/** Read access, folders by name. */
	grants: Grant[]
	deployers: string[]
	/** Folder whose default user becomes the account for every new item in it. */
	defaultFor?: string
}

// The deploy dialog's run-as field needs an email for the account before it exists; the deploy
// replaces it with the real one, which only the server's creation knows.
export const PENDING_EMAIL_DOMAIN = 'pending.sa.wm.dev'

const pending = new SvelteMap<string, PendingServiceAccount>()
const key = (workspace: string, permissionedAs: string) => `${workspace}:${permissionedAs}`

export function setPendingServiceAccount(workspace: string, account: PendingServiceAccount): void {
	pending.set(key(workspace, `u/${account.username}`), account)
}

export function getPendingServiceAccount(
	workspace: string,
	permissionedAs: string | undefined
): PendingServiceAccount | undefined {
	return permissionedAs ? pending.get(key(workspace, permissionedAs)) : undefined
}

const TTL_MS = 60_000
const asked = new Map<string, { at: number; accounts: Promise<ServiceAccount[]> }>()

async function loadServiceAccounts(workspace: string): Promise<ServiceAccount[]> {
	const [users, folders] = await Promise.all([
		UserService.listUsers({ workspace }),
		FolderService.listFolders({ workspace, perPage: 1000 }).catch(() => [])
	])
	return users
		.filter((u) => u.is_service_account && !u.disabled)
		.map((u) => {
			const principal = `u/${u.username}`
			return {
				username: u.username,
				email: u.email,
				folders: folders
					.filter(
						(f) =>
							f.owners.includes(principal) ||
							Object.keys((f.extra_perms ?? {}) as Record<string, boolean>).includes(principal)
					)
					.map((f) => f.name)
			}
		})
}

function fetchServiceAccounts(workspace: string): Promise<ServiceAccount[]> {
	const hit = asked.get(workspace)
	if (hit && Date.now() - hit.at < TTL_MS) return hit.accounts
	const accounts = loadServiceAccounts(workspace).catch(() => [])
	asked.set(workspace, { at: Date.now(), accounts })
	return accounts
}

/** The workspace's service accounts with the folders each reads, followed by the ones picked in
 *  this tab that a deploy has yet to create. */
export function useServiceAccounts(workspace: () => string | undefined) {
	const accounts = resource(workspace, (ws) =>
		ws ? fetchServiceAccounts(ws) : Promise.resolve([])
	)
	return {
		get current(): ServiceAccount[] {
			const ws = workspace()
			const existing = accounts.current ?? []
			const waiting = [...pending.entries()]
				.filter(
					([k, p]) => k.startsWith(`${ws}:`) && !existing.some((a) => a.username === p.username)
				)
				.map(([, p]) => ({
					username: p.username,
					email: `${p.username}@${PENDING_EMAIL_DOMAIN}`,
					folders: p.grants.filter((g) => g.kind === 'folder').map((g) => g.path),
					items: p.grants.filter((g) => g.kind !== 'folder').map((g) => g.path),
					pending: true
				}))
			return [...waiting, ...existing]
		},
		get loading(): boolean {
			return accounts.loading
		}
	}
}

/**
 * Creates the pending account `permissionedAs` names and gives it what was chosen with it, and
 * returns its email; `undefined` when nothing is pending under that name. Each step can rerun, so
 * a deploy that fails partway retries against the account it already made.
 */
export async function createPendingServiceAccount(
	workspace: string,
	permissionedAs: string | undefined
): Promise<string | undefined> {
	const account = getPendingServiceAccount(workspace, permissionedAs)
	if (!account || !permissionedAs) return undefined
	const { username } = account
	const exists = await UserService.getUser({ workspace, username }).then(
		() => true,
		() => false
	)
	if (!exists) {
		await WorkspaceService.createServiceAccount({
			workspace,
			requestBody: { username, operator: true }
		})
	}
	for (const grant of account.grants) {
		await setGrant(workspace, grant.kind, grant.path, permissionedAs, 'read')
	}
	await GranularAclService.setServiceAccountDeployers({
		workspace,
		requestBody: { principal: permissionedAs, deployers: account.deployers }
	})
	if (account.defaultFor) {
		const folder = await FolderService.getFolder({ workspace, name: account.defaultFor })
		const rules = folder.default_permissioned_as ?? []
		if (!rules.some((r) => r.permissioned_as === permissionedAs)) {
			// Appended: a rule the folder already has for some of its paths keeps precedence.
			await FolderService.updateFolder({
				workspace,
				name: account.defaultFor,
				requestBody: {
					default_permissioned_as: [...rules, { path_glob: '**', permissioned_as: permissionedAs }]
				}
			})
		}
	}
	const { email } = await UserService.getUser({ workspace, username })
	pending.delete(key(workspace, permissionedAs))
	asked.delete(workspace)
	forgetDeployAs(workspace)
	return email
}

/** Creates the pending account an app's policy runs as, and points the policy at its email.
 *  False, after telling the user, when that failed and the deploy must not go ahead. */
export async function createPolicyServiceAccount(
	workspace: string,
	policy: { on_behalf_of?: string; on_behalf_of_email?: string }
): Promise<boolean> {
	try {
		const email = await createPendingServiceAccount(workspace, policy.on_behalf_of)
		if (email) policy.on_behalf_of_email = email
		return true
	} catch (e) {
		sendUserToast(`Could not create ${policy.on_behalf_of}: ${e?.body ?? e}`, true)
		return false
	}
}
