/**
 * Supabase Management API calls, proxied through Windmill's backend.
 *
 * The Management API sends no access-control-allow-origin, so the browser cannot call it
 * directly -- every request below goes through /api/oauth/*, which forwards the user's OAuth
 * access token.
 */

export type SupabaseOrg = { id: string; name: string; slug?: string }

export type SupabaseProject = {
	id: string
	name: string
	region: string
	status?: string
	database?: { host: string }
}

/** Region codes accepted by region_selection. */
export const SUPABASE_REGIONS = [
	'us-east-1',
	'us-west-1',
	'eu-central-1',
	'eu-west-1',
	'eu-west-3',
	'ap-southeast-1',
	'ap-northeast-1'
]

export const DEFAULT_SUPABASE_REGION = 'eu-central-1'

function headers(token: string): HeadersInit {
	return { 'Content-Type': 'application/json', 'X-Supabase-Token': token }
}

async function unwrap(res: Response, what: string): Promise<any> {
	if (!res.ok) {
		const body = await res.text()
		throw new Error(`${what}: ${body || res.statusText}`)
	}
	return res.json()
}

export async function listSupabaseOrgs(token: string): Promise<SupabaseOrg[]> {
	const res = await fetch('/api/oauth/list_supabase_orgs', { headers: headers(token) })
	return unwrap(res, 'Could not list your Supabase organizations')
}

export async function listSupabaseProjects(token: string): Promise<SupabaseProject[]> {
	const res = await fetch('/api/oauth/list_supabase', { headers: headers(token) })
	return unwrap(res, 'Could not list your Supabase projects')
}

/** organization_slug is what create takes; older payloads only carry an id. */
export function orgSlug(org: SupabaseOrg): string {
	return org.slug ?? org.id
}

/**
 * Supabase never lets a database password be read back, so the only way to know it is to be
 * the one who set it: db_pass is an input to project creation.
 */
export function generateDbPassword(): string {
	const charset = 'abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789'
	const values = new Uint32Array(32)
	crypto.getRandomValues(values)
	return Array.from(values, (v) => charset[v % charset.length]).join('')
}

export async function createSupabaseProject(
	token: string,
	args: { name: string; organizationSlug: string; region: string; dbPass: string }
): Promise<SupabaseProject> {
	const res = await fetch('/api/oauth/create_supabase_project', {
		method: 'POST',
		headers: headers(token),
		body: JSON.stringify({
			name: args.name,
			organization_slug: args.organizationSlug,
			db_pass: args.dbPass,
			// region_selection is { type: 'specific' | 'smartGroup', code }. Neither the published
			// docs nor the OpenAPI spec describe it correctly (they give `kind`/`region` and
			// `primary`) -- this shape comes from the API's own validation errors, so do not
			// "correct" it against the documentation.
			region_selection: { type: 'specific', code: args.region }
		})
	})
	return unwrap(res, 'Supabase refused to create the project')
}

/**
 * Creation returns immediately with the project still coming up, so the pooler is not
 * reachable yet. Poll until Supabase reports it healthy before trying to connect.
 */
export async function waitUntilSupabaseHealthy(
	token: string,
	projectId: string,
	onStatus?: (status: string | undefined) => void,
	attempts = 60
): Promise<SupabaseProject> {
	for (let i = 0; i < attempts; i++) {
		await new Promise((r) => setTimeout(r, 5000))
		let list: SupabaseProject[]
		try {
			list = await listSupabaseProjects(token)
		} catch {
			continue
		}
		const project = list?.find?.((p) => p.id === projectId)
		if (project?.status === 'ACTIVE_HEALTHY') return project
		onStatus?.(project?.status)
	}
	throw new Error('Timed out waiting for the project to become reachable')
}

/**
 * https://github.com/orgs/supabase/discussions/17817
 * host is `aws-0-${region}.pooler.supabase.com`, user is `postgres.${id}`. The direct host is
 * IPv6-only on free projects, which is why this targets the pooler rather than
 * `database.host` from the API.
 */
export function supabaseResourceValue(project: SupabaseProject, passwordVarPath: string) {
	return {
		host: `aws-0-${project.region}.pooler.supabase.com`,
		user: `postgres.${project.id}`,
		port: 5432,
		dbname: 'postgres',
		sslmode: 'prefer',
		password: `$var:${passwordVarPath}`
	}
}
