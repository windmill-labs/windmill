/**
 * What a database lets the data table's role do, answered by the worker rather than by the API
 * server.
 *
 * It runs as a preview job for the same reason `TestConnection` does: a job goes through the
 * worker's Postgres executor, so IAM and Azure workload identity authenticate as the worker
 * will when a real query runs. A connection opened from the API server proves something about
 * the API server, which is a different machine with a different identity.
 *
 * Postgres composes the suggested statements itself through `format('%I')`, so identifier
 * quoting stays where it is already implemented.
 */

import { JobService, type Preview, type TestDataTableConnectionResponse } from '$lib/gen'
import { tryEvery } from '$lib/utils'

const PRIVILEGES = `SELECT current_user AS usr,
       current_schema() AS sch,
       has_schema_privilege(current_schema(), 'CREATE') AS can_create_table,
       has_database_privilege(current_database(), 'CREATE') AS can_create_schema,
       to_regclass('_wm_migrations') IS NOT NULL AS has_migrations_table,
       -- A role whose search_path names no valid schema has a NULL current_schema(), and
       -- format('%I', NULL) raises rather than returning NULL, which would fail the whole
       -- query on the one case fix_search_path exists to report.
       CASE WHEN current_schema() IS NULL THEN NULL
            ELSE format('GRANT CREATE ON SCHEMA %I TO %I', current_schema(), current_user)
       END AS grant_schema,
       format('GRANT CREATE ON DATABASE %I TO %I', current_database(), current_user) AS grant_database,
       format('ALTER ROLE %I SET search_path = public', current_user) AS fix_search_path`

type Row = {
	usr?: string
	sch?: string | null
	can_create_table?: boolean
	can_create_schema?: boolean
	has_migrations_table?: boolean
	grant_schema?: string | null
	grant_database?: string | null
	fix_search_path?: string | null
}

/**
 * `database` is whatever a Postgres step takes: the resource value, or a `$res:` path the
 * worker resolves. Throws with the database's own message when the query fails, and after
 * `timeout` when no worker picks the job up.
 */
export async function probeDatatableConnection(
	workspace: string,
	database: Record<string, any> | string,
	// Longer than the 20s the worker allows its own Postgres connect, or a host that accepts
	// the connection and never answers -- a firewall with no rule for the workers, which this
	// check exists to catch -- is cancelled first and reported as a missing worker.
	timeout = 30000
): Promise<TestDataTableConnectionResponse> {
	const job = await JobService.runScriptPreview({
		workspace,
		requestBody: {
			path: 'testConnection: datatable',
			language: 'postgresql' as Preview['language'],
			content: PRIVILEGES,
			args: { database }
		}
	})

	let completed: Awaited<ReturnType<typeof JobService.getCompletedJob>> | undefined = undefined
	await tryEvery({
		tryCode: async () => {
			completed = await JobService.getCompletedJob({ workspace, id: job })
		},
		timeoutCode: async () => {
			await JobService.cancelQueuedJob({
				workspace,
				id: job,
				requestBody: { reason: 'The connection check did not start' }
			}).catch(() => {})
		},
		interval: 500,
		timeout
	})

	if (!completed) {
		throw new Error(
			'The connection check did not run. Is a worker listening to the postgresql tag available?'
		)
	}
	const done = completed as { success: boolean; result?: any }
	if (!done.success) {
		throw new Error(done.result?.error?.message ?? 'Could not connect to the database')
	}

	const row: Row = (Array.isArray(done.result) ? done.result[0] : done.result) ?? {}
	// Suggested only where the privilege is actually missing; Postgres returns NULL for a
	// statement it could not name, which is the case where no grant would help anyway.
	const suggested_grants = [
		row.can_create_table ? undefined : (row.grant_schema ?? undefined),
		row.can_create_schema ? undefined : (row.grant_database ?? undefined)
	].filter((s): s is string => !!s)

	return {
		user: row.usr ?? '',
		schema: row.sch ?? null,
		can_create_table: !!row.can_create_table,
		can_create_schema: !!row.can_create_schema,
		migrations_table_exists: !!row.has_migrations_table,
		suggested_grants,
		suggested_search_path: row.sch ? undefined : (row.fix_search_path ?? undefined)
	}
}
