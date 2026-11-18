import { z } from 'zod'

/**
 * The rich-result kinds a ```wm-display fence may hold, and the `chart` kind a script result may
 * take. The schemas validate what the model writes, gate `chart` in `DisplayResult`, and are
 * exported as JSON Schema into the model's guidance (`displayCatalog.gen.test.ts`).
 *
 * The fence only admits kinds that issue no request and run no markup: it renders on the public
 * shared-session page, which promises to fetch nothing. Do not add `html`, `svg`, images, `map`,
 * `markdown` (its images fetch), `file`, `s3object`, `approval` or `pdf` to `fenceSchema`.
 */

export const MAX_CHART_ROWS = 5000
/** One per categorical palette slot in `ChartResult`: a ninth series would need a cycled hue. */
export const MAX_CHART_SERIES = 8
export const MAX_TABLE_ROWS = 5000

const cell = z.union([z.string(), z.number(), z.boolean(), z.null()])

const row = z
	.record(z.string(), cell)
	.describe('One row: column name to value (string, number, boolean or null)')

export const chartBodySchema = z
	.strictObject({
		type: z
			.enum(['bar', 'line', 'pie', 'scatter'])
			.describe(
				'bar or line for categories or time, pie for parts of a whole, scatter for two numeric columns'
			),
		title: z.string().optional(),
		data: z
			.array(row)
			.min(1)
			.max(MAX_CHART_ROWS)
			.describe('The rows to plot, the same shape as a table-row-object result'),
		x: z
			.string()
			.describe(
				'Column holding the category, label or date of each row (bar, line, pie), or the numeric x value (scatter)'
			),
		y: z
			.union([z.string(), z.array(z.string()).min(1).max(MAX_CHART_SERIES)])
			.describe('Numeric column, or columns, to plot: one series each. A pie takes exactly one'),
		stacked: z.boolean().optional().describe('bar and line: stack the series'),
		horizontal: z.boolean().optional().describe('bar: draw the bars horizontally'),
		x_label: z.string().optional(),
		y_label: z.string().optional()
	})
	.superRefine((chart, ctx) => {
		const ys = typeof chart.y === 'string' ? [chart.y] : chart.y
		if (chart.type === 'pie' && ys.length !== 1) {
			ctx.addIssue({
				code: 'custom',
				path: ['y'],
				message: 'a pie chart takes exactly one y column'
			})
		}
		const columns = new Set(chart.data.flatMap((r) => Object.keys(r)))
		for (const [key, column] of [['x', chart.x], ...ys.map((c) => ['y', c] as const)] as const) {
			if (!columns.has(column)) {
				ctx.addIssue({
					code: 'custom',
					path: [key],
					message: `column "${column}" is not in any row of data`
				})
			}
		}
		for (const column of ys) {
			const bad = chart.data.findIndex(
				(r) => r[column] !== undefined && r[column] !== null && typeof r[column] !== 'number'
			)
			if (bad !== -1) {
				ctx.addIssue({
					code: 'custom',
					path: ['data', bad, column],
					message: `y column "${column}" must hold numbers`
				})
			}
		}
		if (chart.type === 'scatter') {
			const bad = chart.data.findIndex((r) => typeof r[chart.x] !== 'number')
			if (bad !== -1) {
				ctx.addIssue({
					code: 'custom',
					path: ['data', bad, chart.x],
					message: `a scatter chart's x column "${chart.x}" must hold numbers`
				})
			}
		}
	})

export type ChartBody = z.infer<typeof chartBodySchema>

export const chartSchema = z.strictObject({ chart: chartBodySchema })

const tableRowObjectSchema = z.strictObject({
	'table-row-object': z
		.array(row)
		.min(1)
		.max(MAX_TABLE_ROWS)
		.describe('Rows as objects; the columns are the union of their keys')
})

const tableColSchema = z.strictObject({
	'table-col': z
		.record(z.string(), z.array(cell).max(MAX_TABLE_ROWS))
		.describe('Columns: column name to its values, all the same length')
})

const tableRowSchema = z.strictObject({
	'table-row': z
		.array(z.array(cell))
		.min(2)
		.max(MAX_TABLE_ROWS)
		.describe('Rows as arrays; the first row holds the column names')
})

const fenceItemSchema = z.union([chartSchema, tableRowObjectSchema, tableColSchema, tableRowSchema])

export const fenceSchema = z.union([
	fenceItemSchema,
	z.strictObject({
		render_all: z
			.array(fenceItemSchema)
			.min(1)
			.max(10)
			.describe('Several items shown one after another')
	})
])

export type FenceResult = z.infer<typeof fenceSchema>
export type FenceItem = z.infer<typeof fenceItemSchema>
export type FenceTable = Exclude<FenceItem, { chart: unknown }>

/** The items a fence value renders, in order. */
export function fenceItems(value: FenceResult): FenceItem[] {
	return 'render_all' in value ? value.render_all : [value]
}

type Cell = string | number | boolean | null

/** The columns and rows of a catalog table value, in the order the table renders them. */
export function tableRows(item: FenceTable): { columns: string[]; rows: Cell[][] } {
	if ('table-row' in item) {
		const [header, ...rows] = item['table-row']
		return { columns: header.map((c) => String(c ?? '')), rows }
	}
	if ('table-col' in item) {
		const cols = item['table-col']
		const columns = Object.keys(cols)
		const length = Math.max(0, ...columns.map((c) => cols[c].length))
		return {
			columns,
			rows: Array.from({ length }, (_, i) => columns.map((c) => cols[c][i] ?? null))
		}
	}
	const objects = item['table-row-object']
	const columns = [...new Set(objects.flatMap((o) => Object.keys(o)))]
	return { columns, rows: objects.map((o) => columns.map((c) => o[c] ?? null)) }
}

export function isChartResult(result: unknown): result is { chart: ChartBody } {
	return chartSchema.safeParse(result).success
}

/** The fence kinds, as `feature_usage` keys: a closed set. */
export type FenceKind = 'chart' | 'table' | 'render_all'

export function fenceKind(result: FenceResult): FenceKind {
	if ('render_all' in result) return 'render_all'
	if ('chart' in result) return 'chart'
	return 'table'
}

export type FenceParse = { ok: true; result: FenceResult } | { ok: false; error: string }

export function parseFence(source: string): FenceParse {
	let json: unknown
	try {
		json = JSON.parse(source)
	} catch (e) {
		return { ok: false, error: `Not valid JSON: ${(e as Error).message}` }
	}
	const parsed = fenceSchema.safeParse(json)
	if (parsed.success) return { ok: true, result: parsed.data }
	return { ok: false, error: describeFenceError(json, parsed.error) }
}

const FENCE_KEYS = ['chart', 'table-row-object', 'table-col', 'table-row', 'render_all']

// A union's own error lists every branch's failure; report the branch the object's key names.
function describeFenceError(json: unknown, error: z.ZodError): string {
	if (json && typeof json === 'object' && !Array.isArray(json)) {
		const keys = Object.keys(json)
		if (keys.length !== 1 || !FENCE_KEYS.includes(keys[0])) {
			return `Expected an object with exactly one of the keys ${FENCE_KEYS.join(', ')}; got ${keys.length ? keys.join(', ') : 'no key'}`
		}
		const key = keys[0]
		const branch =
			key === 'render_all'
				? fenceSchema.options[1]
				: fenceItemSchema.options.find((o) => key in o.shape)
		const inner = branch?.safeParse(json)
		if (inner && !inner.success) return z.prettifyError(inner.error)
	}
	return z.prettifyError(error)
}
