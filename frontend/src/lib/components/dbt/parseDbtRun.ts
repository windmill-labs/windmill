export type DbtNode = {
	unique_id: string
	status: string
	execution_time?: number
	rows_affected?: number
	relation_name?: string
	message?: string
}

export type DbtRun = {
	engine?: string
	engine_version?: string
	command?: string
	totals?: { total?: number; success?: number; error?: number; warn?: number; skipped?: number }
	nodes?: DbtNode[]
}

/** The engines the worker stamps on a result. This is the discriminator: a
 *  `{nodes, totals}` shape alone is one an ordinary script can return, and it
 *  would then be rendered as somebody's dbt run. */
const ENGINES = ['dbt-core-1x', 'dbt-core-2x', 'fusion']

function asDbtRun(v: unknown): DbtRun | undefined {
	if (!v || typeof v !== 'object') return undefined
	const o = v as Record<string, unknown>
	return ENGINES.includes(o.engine as string) &&
		Array.isArray(o.nodes) &&
		o.totals != undefined &&
		typeof o.totals === 'object'
		? (o as DbtRun)
		: undefined
}

/**
 * The dbt invocation a job result describes, if it describes one.
 *
 * On success the result IS the run. On failure the worker puts the same JSON in
 * the error message after the exit-status line, and that is the case worth
 * rendering: the failing node is what the user came for.
 */
export function parseDbtRun(result: any): DbtRun | undefined {
	const direct = asDbtRun(result)
	if (direct) return direct
	const msg = result?.error?.message
	if (typeof msg !== 'string') return undefined
	const start = msg.indexOf('{')
	if (start === -1) return undefined
	try {
		return asDbtRun(JSON.parse(msg.slice(start)))
	} catch {
		return undefined
	}
}

/**
 * Ordering rank of a node's status: 0 failed, 1 warned, 2 skipped, 3 passed.
 *
 * `partial success` is dbt's word for a node that built but whose tests failed.
 * The worker counts it in `totals.error` and a retry redoes it, so showing it
 * green would contradict the job's own outcome and hide the message saying why.
 */
export function statusRank(status: string): number {
	switch (status.trim().toLowerCase()) {
		case 'error':
		case 'fail':
		case 'runtime error':
		case 'partial success':
			return 0
		case 'warn':
			return 1
		case 'skipped':
			return 2
		default:
			return 3
	}
}

/**
 * The kind and name behind a dbt `unique_id`, which dbt builds as
 * `<resource_type>.<package>.<name>`. A generic test's name carries a trailing
 * hash dbt adds for uniqueness; it is noise in a run summary.
 */
export function splitUniqueId(uniqueId: string): { kind: string; name: string } {
	const parts = uniqueId.split('.')
	const kind = parts[0] ?? ''
	let name = parts.slice(2).join('.')
	if (kind === 'test') name = name.replace(/\.[0-9a-f]{6,}$/, '')
	return { kind, name: name || uniqueId }
}
