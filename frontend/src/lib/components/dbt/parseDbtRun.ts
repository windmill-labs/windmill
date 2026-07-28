export type DbtNode = {
	unique_id: string
	status: string
	/** Windmill's stable word for the same result, published beside dbt's own.
	 *  Preferred wherever a decision is made: `status` is dbt's vocabulary and
	 *  dbt may rename it. */
	outcome?: DbtOutcome
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
	/** The arguments the run actually used, as submitted. A `dbt retry` restores
	 *  the failed run's arguments inside the worker, so the retry job's own args
	 *  are only `{dbt_command: 'retry'}` — this is the sole way to recover what
	 *  it really ran with. */
	invocation_args?: Record<string, unknown>
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

/** Ordering rank of a node's status: 0 failed, 1 warned, 2 skipped, 3 passed. */
export function statusRank(status: string, outcome?: DbtOutcome): number {
	switch (outcome ?? classifyStatus(status)) {
		case 'failed':
			return 0
		case 'warned':
			return 1
		case 'skipped':
			return 2
		default:
			return 3
	}
}

/** The worker's stable vocabulary for a node result, published as `outcome`. */
export type DbtOutcome = 'started' | 'passed' | 'failed' | 'warned' | 'skipped' | 'no_op' | 'unknown'

/**
 * dbt's node status, reduced to the outcomes the UI distinguishes.
 *
 * Only for results that predate `outcome`, or for the live event stream, which
 * carries dbt's word alone. Anything holding a node from a job result should
 * read `outcome` instead — that is the field the worker publishes precisely so
 * this mapping is not the contract.
 */
function classifyStatus(
	status: string
): 'started' | 'passed' | 'failed' | 'warned' | 'skipped' | 'other' {
	// `partial success` is dbt's word for a node that built but whose tests
	// failed. The worker counts it in `totals.error` and a retry redoes it, so
	// showing it green would contradict the job's own outcome.
	switch (status.trim().toLowerCase()) {
		case 'started':
			return 'started'
		case 'success':
		case 'pass':
			return 'passed'
		case 'error':
		case 'fail':
		case 'runtime error':
		case 'partial success':
			return 'failed'
		case 'warn':
			return 'warned'
		case 'skipped':
			return 'skipped'
		default:
			return 'other'
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

/**
 * What a node's status says happened to the relation it builds, or `undefined`
 * when it says nothing.
 *
 * Mirrors the worker's `classify_status`, and must keep mirroring it: the two
 * decide the same thing about the same string, one for the record it writes and
 * one for the colour drawn over it. `warn`, `skipped` and `no-op` leave the
 * relation untouched, so they get no colour rather than a misleading one.
 */
export function relationOutcome(
	status: string,
	outcome?: DbtOutcome
): 'running' | 'materialized' | 'failed' | undefined {
	switch (outcome ?? classifyStatus(status)) {
		case 'started':
			return 'running'
		case 'passed':
			return 'materialized'
		case 'failed':
			return 'failed'
		// `warn`, `skipped` and `no-op` say nothing about the relation: nothing
		// was written, so its state is whatever the last run left.
		default:
			return undefined
	}
}

/**
 * dbt's `relation_name` split into its parts, honouring quoting.
 *
 * Mirrors the worker's `split_relation`: `"`, `` ` `` and `[` open a quoted
 * identifier, and a `.` inside one is part of the name. Splitting on every `.`
 * turns `"wh"."analytics.v2"."orders"` into a relation called `orders` in a
 * schema called `v2` — a table that does not exist.
 */
export function splitRelation(relation: string): string[] {
	const parts: string[] = []
	let current = ''
	let quote: string | undefined
	for (const c of relation) {
		if (quote !== undefined) {
			if (c === quote || (quote === '[' && c === ']')) quote = undefined
			else current += c
		} else if (c === '"' || c === '`' || c === '[') {
			quote = c
		} else if (c === '.') {
			parts.push(current)
			current = ''
		} else {
			current += c
		}
	}
	parts.push(current)
	return parts.map((p) => p.trim())
}
