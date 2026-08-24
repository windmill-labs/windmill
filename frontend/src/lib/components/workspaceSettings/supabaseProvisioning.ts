/**
 * Supabase Management API calls, proxied through Windmill's backend.
 *
 * The Management API sends no access-control-allow-origin, so the browser cannot call it
 * directly -- every request below goes through /api/oauth/*, which forwards the user's OAuth
 * access token.
 */

import { DEFAULT_SSLMODE } from '$lib/utils/postgresConnectionString'
import { base } from '$lib/base'
import { oauthStore } from '$lib/stores'
import { get } from 'svelte/store'

export type SupabaseOrg = { id: string; slug?: string; name: string }

export type SupabaseProject = {
	/** `id` is Supabase's deprecated spelling of `ref`; both are sent today. */
	id?: string
	ref?: string
	name: string
	region: string
	status?: string
	organization_slug?: string
	organization_id?: string
	database?: { host: string }
}

/** One Supavisor endpoint of a project. A project has one per mode and replica. */
export type SupabasePooler = {
	database_type: 'PRIMARY' | 'READ_REPLICA'
	pool_mode: 'transaction' | 'session'
	db_user: string
	db_host: string
	db_port: number
	db_name: string
}

export type SupabaseConnectionMode = 'session' | 'direct'

/** Supabase deprecated `id` in favour of `ref`, and still sends both. */
export function projectRef(project: SupabaseProject): string {
	return project.ref ?? project.id ?? ''
}

export function projectOrg(project: SupabaseProject): string | undefined {
	return project.organization_slug ?? project.organization_id
}

/** Region codes accepted by region_selection, with the names Supabase shows for them. */
export const SUPABASE_REGIONS: { code: string; label: string }[] = [
	{ code: 'us-east-1', label: 'East US (N. Virginia)' },
	{ code: 'us-west-1', label: 'West US (N. California)' },
	{ code: 'eu-central-1', label: 'Central EU (Frankfurt)' },
	{ code: 'eu-west-1', label: 'West EU (Ireland)' },
	{ code: 'eu-west-3', label: 'West EU (Paris)' },
	{ code: 'ap-southeast-1', label: 'Southeast Asia (Singapore)' },
	{ code: 'ap-northeast-1', label: 'Northeast Asia (Tokyo)' }
]

export const DEFAULT_SUPABASE_REGION = 'eu-central-1'

function headers(token: string): HeadersInit {
	return { 'Content-Type': 'application/json', 'X-Supabase-Token': token }
}

async function unwrap(res: Response, what: string): Promise<any> {
	if (!res.ok) {
		// Supabase access tokens are short-lived while `oauthStore` lasts as long as the tab, so
		// a stale one otherwise leaves every caller "authorized" and unable to reach the button
		// that would fix it. Forgetting it here is what puts Connect back on screen.
		if (res.status === 401) oauthStore.set(undefined)
		const body = await res.text()
		throw new Error(`${what}: ${supabaseErrorMessage(body) || res.statusText}`)
	}
	return res.json()
}

/**
 * Supabase answers with `{ message }` or `{ error }` and occasionally plain text.
 * Surfacing the raw body puts a JSON blob in front of the user, so unwrap it to
 * the sentence inside.
 */
export function supabaseErrorMessage(body: string): string {
	try {
		const parsed = JSON.parse(body)
		return parsed?.message ?? parsed?.error ?? parsed?.msg ?? body
	} catch {
		return body
	}
}

export async function listSupabaseOrgs(token: string): Promise<SupabaseOrg[]> {
	const res = await fetch(`${base}/api/oauth/list_supabase_orgs`, { headers: headers(token) })
	return unwrap(res, 'Could not list your Supabase organizations')
}

export async function listSupabaseProjects(token: string): Promise<SupabaseProject[]> {
	const res = await fetch(`${base}/api/oauth/list_supabase`, { headers: headers(token) })
	return unwrap(res, 'Could not list your Supabase projects')
}

/** Plan of one organization, which the list endpoint does not carry. */
export async function getSupabaseOrgPlan(token: string, slug: string): Promise<string | undefined> {
	try {
		const res = await fetch(`${base}/api/oauth/get_supabase_org/${slug}`, {
			headers: headers(token)
		})
		if (!res.ok) return undefined
		return (await res.json())?.plan
	} catch {
		return undefined
	}
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
	const res = await fetch(`${base}/api/oauth/create_supabase_project`, {
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
		} catch (err) {
			// A transient failure is worth another poll; an expired token is not -- retrying it
			// burns five minutes and then reports a timeout, which names the wrong problem.
			if (!get(oauthStore)?.access_token) throw err
			continue
		}
		const project = list?.find?.((p) => projectRef(p) === projectId)
		if (project?.status === 'ACTIVE_HEALTHY') return project
		onStatus?.(project?.status)
	}
	throw new Error('Timed out waiting for the project to become reachable')
}

/**
 * The session-mode Supavisor endpoint of the project's primary database.
 *
 * Which pooler a project sits behind is assigned by Supabase, not derived from its
 * region: constructing `aws-0-<region>.pooler.supabase.com` is wrong for every project
 * that landed on another one, and the resulting resource never connects.
 */
export async function getSupabasePooler(token: string, projectId: string): Promise<SupabasePooler> {
	const res = await fetch(`${base}/api/oauth/get_supabase_pooler/${projectId}`, {
		headers: headers(token)
	})
	const configs: SupabasePooler[] = await unwrap(res, 'Could not read the connection details')
	const primary = configs.filter((c) => c.database_type === 'PRIMARY')
	const pooler = primary.find((c) => c.pool_mode === 'session') ?? primary[0] ?? configs[0]
	if (!pooler) throw new Error('Supabase returned no connection details for this project')
	return pooler
}

export type SupabaseConnection = {
	mode: SupabaseConnectionMode
	pooler?: SupabasePooler
	/** Why session pooling was asked for and not used. Absent when nothing was given up. */
	unavailable?: string
}

/**
 * The endpoint a project should be reached through, degrading rather than failing. Reading the
 * pooler config needs the `database_pooling_config_read` scope, which an instance's OAuth app
 * may not have. A direct connection still works where the workers have IPv6, so fall back to
 * it and say so.
 */
export async function resolveSupabaseConnection(
	token: string,
	project: SupabaseProject,
	mode: SupabaseConnectionMode
): Promise<SupabaseConnection> {
	if (mode !== 'session') return { mode }
	try {
		return { mode, pooler: await getSupabasePooler(token, projectRef(project)) }
	} catch (err) {
		return { mode: 'direct', unavailable: err instanceof Error ? err.message : String(err) }
	}
}

/** The resource value for a project, given the endpoint it should connect through. */
export function supabaseResourceValue(
	project: SupabaseProject,
	passwordVarPath: string,
	connection: { mode: SupabaseConnectionMode; pooler?: SupabasePooler }
) {
	const direct = connection.mode === 'direct' || !connection.pooler
	return {
		host: direct
			? (project.database?.host ?? `db.${projectRef(project)}.supabase.co`)
			: connection.pooler!.db_host,
		user: direct ? 'postgres' : connection.pooler!.db_user,
		port: direct ? 5432 : connection.pooler!.db_port,
		dbname: direct ? 'postgres' : connection.pooler!.db_name,
		// Supabase terminates TLS on every endpoint it hands out, and this connection carries a
		// generated password, so there is no reason to leave a plaintext fallback open.
		sslmode: DEFAULT_SSLMODE,
		password: `$var:${passwordVarPath}`,
		// Resource forms fill in every unset property from the schema as soon as they render,
		// so a postgresql resource saved without these comes up already modified -- and saves a
		// draft -- the first time anyone opens it. Write them here so opening one is a no-op.
		// (accept_invalid_certs renders conditionally and is not seeded, so it stays out.)
		region: '',
		root_certificate_pem: '',
		use_iam_auth: false
	}
}
