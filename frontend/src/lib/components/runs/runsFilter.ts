import type { JobTriggerKind } from '$lib/gen'
import z from 'zod'

export const runsFiltersSchema = z.object({
	path: z.string().nullable().default(null),
	worker: z.string().nullable().default(null),
	user: z.string().nullable().default(null),
	folder: z.string().nullable().default(null),
	label: z.string().nullable().default(null),
	concurrency_key: z.string().nullable().default(null),
	tag: z.string().nullable().default(null),
	allow_wildcards: z.boolean().default(false),
	show_future_jobs: z.boolean().default(true),
	success: z
		.enum(['running', 'suspended', 'waiting', 'success', 'failure'])
		.nullable()
		.default(null),
	show_skipped: z.boolean().default(false),
	show_schedules: z.boolean().default(true),
	schedule_path: z.string().nullable().default(null),
	job_kinds: z.string().default('runs'),
	all_workspaces: z.boolean().default(false),
	arg: z.string().default(''),
	result: z.string().default(''),
	job_trigger_kind: z
		.string()
		.transform((s) => s as JobTriggerKind)
		.nullable()
		.default(null),
	per_page: z.number().default(1000)
})
export type RunsFilters = z.infer<typeof runsFiltersSchema>
