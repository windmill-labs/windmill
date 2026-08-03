import { z } from 'zod'
import { scheduleRequestSchema } from './workspaceToolsZod.gen'

// Fields common enough that a lookup round trip would cost more than carrying them.
// The rest reach the same validation through `advanced`, whose shape get_schedule_schema
// serves on demand: `retry` alone is a quarter of this schema and is asked for far more
// rarely than a plain cron.
const COMMON_SCHEDULE_FIELDS = [
	'path',
	'schedule',
	'timezone',
	'script_path',
	'is_flow',
	'args',
	'enabled',
	'summary',
	'description',
	'on_failure',
	'on_recovery',
	'on_success'
] as const

/** Fields the caller supplies itself, so the model must neither set nor see them. */
export type ScheduleToolOptions = { hidden?: readonly string[] }

function commonFields({ hidden = [] }: ScheduleToolOptions): string[] {
	return COMMON_SCHEDULE_FIELDS.filter((field) => !hidden.includes(field))
}

/** The `advanced` shape get_schedule_schema serves: everything not already inline. */
export function advancedScheduleShape(opts: ScheduleToolOptions = {}) {
	const inline = new Set([...commonFields(opts), ...(opts.hidden ?? [])])
	const full = z.toJSONSchema(scheduleRequestSchema) as { properties?: Record<string, unknown> }
	return {
		type: 'object',
		properties: Object.fromEntries(
			Object.entries(full.properties ?? {}).filter(([field]) => !inline.has(field))
		)
	}
}

export function buildScheduleToolSchema(opts: ScheduleToolOptions = {}) {
	const advancedNames = Object.keys(advancedScheduleShape(opts).properties).join(', ')
	return scheduleRequestSchema
		.pick(Object.fromEntries(commonFields(opts).map((f) => [f, true])) as any)
		.extend({
			advanced: z
				.record(z.string(), z.any())
				.optional()
				.describe(
					`Less common schedule options (${advancedNames}). Call get_schedule_schema for their exact shape.`
				)
		})
}

/**
 * Explains keys a strip-mode parse discarded. A mis-shaped real field is recoverable by
 * fetching its schema; a key that is not a schedule field at all is not, and pointing
 * the model at the lookup for it would send it somewhere with no answer.
 */
export function describeDroppedScheduleOptions(dropped: string[]): string {
	const fields = new Set(
		Object.keys(
			(z.toJSONSchema(scheduleRequestSchema) as { properties?: Record<string, unknown> })
				.properties ?? {}
		)
	)
	const misshaped = dropped.filter((path) => fields.has(path.split('.')[0]))
	const unknown = dropped.filter((path) => !fields.has(path.split('.')[0]))
	return [
		misshaped.length
			? `These schedule options do not match their schema and would have been dropped: ${misshaped.join(', ')}. Call get_schedule_schema for their exact shape.`
			: '',
		unknown.length ? `These are not schedule fields: ${unknown.join(', ')}.` : ''
	]
		.filter(Boolean)
		.join(' ')
}
