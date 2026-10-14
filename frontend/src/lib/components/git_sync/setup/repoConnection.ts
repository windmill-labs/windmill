import { GitSyncService, ResourceService, VariableService } from '$lib/gen'

/** What a provider form hands back once the user has picked a repository. */
export type RepoConnection = {
	/** The repository URL, never carrying a secret. */
	url: string
	isGithubApp?: boolean
	/** A token for Windmill to store and renew server-side (EE GitLab). */
	heldToken?: string
	/** `url` with the token embedded, filed as a secret variable because the
	 * resource value itself is readable by anyone who can read the resource. */
	tokenUrl?: string
}

/** Username git sends alongside a token: GitHub ignores it for PATs, GitLab
 * requires a non-empty one and documents `oauth2`. */
const TOKEN_USERNAME = { github: 'x-access-token', gitlab: 'oauth2' } as const

export type TokenProvider = keyof typeof TOKEN_USERNAME

/** Parses a pasted repository URL, dropping any credentials it carried. */
export function parseRepoUrl(raw: string): URL | undefined {
	let u: URL
	try {
		u = new URL(raw.trim())
	} catch {
		return undefined
	}
	// https only: the token is embedded in this URL, and plain http would put it on the wire.
	if (u.protocol !== 'https:') return undefined
	if (u.pathname.replace(/\/+$/, '').split('/').filter(Boolean).length < 2) return undefined
	u.username = ''
	u.password = ''
	u.search = ''
	u.hash = ''
	return u
}

export function withToken(url: URL, token: string, provider: TokenProvider): string {
	const u = new URL(url.toString())
	u.username = TOKEN_USERNAME[provider]
	u.password = token.trim()
	return u.toString()
}

/** A resource name derived from the repository, e.g. `acme/infra.git` → `infra`. */
export function repoSlug(url: string): string {
	const last =
		url
			.replace(/\.git\/?$/, '')
			.replace(/\/+$/, '')
			.split('/')
			.pop() ?? ''
	return last.replace(/[^a-zA-Z0-9_]/g, '_').toLowerCase() || 'git_repository'
}

/**
 * Writes everything a connection needs and creates the `git_repository` resource at
 * `path`. A token-bearing URL goes to a secret variable at the same path, which the
 * backend renames along with the resource.
 */
export async function createRepositoryResource(
	workspace: string,
	path: string,
	conn: RepoConnection,
	location: { branch?: string; folder?: string }
): Promise<void> {
	const branch = location.branch?.trim()
	// Relative to the repository root, which is how the sync script joins it.
	const folder = location.folder?.trim().replace(/^\/+|\/+$/g, '')
	let url = conn.url
	let createdVariable = false
	if (conn.tokenUrl) {
		if (await VariableService.existsVariable({ workspace, path })) {
			throw new Error(`A variable already exists at ${path}. Pick another path.`)
		}
		await VariableService.createVariable({
			workspace,
			requestBody: {
				path,
				value: conn.tokenUrl,
				is_secret: true,
				description: `Authenticated URL of the git repository ${conn.url}`
			}
		})
		createdVariable = true
		url = `$var:${path}`
	}
	try {
		if (conn.heldToken) {
			await GitSyncService.setGitCredential({
				workspace,
				requestBody: { repo_url: conn.url, token: conn.heldToken }
			})
		}
		await ResourceService.createResource({
			workspace,
			requestBody: {
				path,
				resource_type: 'git_repository',
				description: `Git repository ${conn.url}`,
				value: {
					url,
					...(branch ? { branch } : {}),
					...(folder ? { folder } : {}),
					is_github_app: !!conn.isGithubApp
				}
			}
		})
	} catch (e) {
		// Otherwise a retry at the same path is refused over this run's own secret.
		if (createdVariable) {
			await VariableService.deleteVariable({ workspace, path }).catch(() => {})
		}
		throw e
	}
}
