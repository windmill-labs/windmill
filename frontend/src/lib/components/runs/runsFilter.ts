import type { JobTriggerKind } from '$lib/gen'
import { Calendar, FolderIcon, UserIcon } from 'lucide-svelte'
import z from 'zod'
import { triggerDisplayNamesMap } from '../triggers/utils'

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

export function buildRunsFilterSearchbarSchema({
	paths,
	usernames,
	folders,
	jobTriggerKinds
}: {
	paths: string[]
	usernames: string[]
	folders: string[]
	jobTriggerKinds: JobTriggerKind[]
}) {
	return {
		min_ts: {
			type: 'date',
			label: 'From',
			icon: Calendar,
			description: 'Only include jobs that completed after this date'
		},
		max_ts: {
			type: 'date',
			label: 'To',
			icon: Calendar,
			description: 'Only include jobs that completed before this date'
		},
		path: {
			type: 'oneof',
			options: paths.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			label: 'Path',
			description: 'Filter by script or flow path'
		},
		user: {
			type: 'oneof',
			options: usernames.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			label: 'User',
			icon: UserIcon,
			description: 'Filter by user who created the job'
		},
		folder: {
			type: 'oneof',
			options: folders.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			label: 'Folder',
			icon: FolderIcon,
			description: 'Filter by folder containing the script or flow'
		},
		label: {
			type: 'string',
			label: 'Label',
			description: 'Filter by custom label attached to jobs'
		},
		tag: { type: 'string', label: 'Tag', description: 'Filter by worker tag' },
		worker: {
			type: 'string',
			label: 'Worker',
			description: 'Filter by specific worker instance'
		},
		schedule_path: {
			type: 'string',
			label: 'Schedule path',
			description: 'Filter by schedule that triggered the job'
		},
		concurrency_key: {
			type: 'string',
			label: 'Concurrency key',
			description: 'Filter by concurrency limit key'
		},
		job_kinds: {
			type: 'oneof',
			options: [
				{ label: 'All', value: 'all' },
				{
					label: 'Runs (default)',
					value: 'runs',
					description:
						'Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they start), they have been triggered through the UI, a schedule or webhook'
				},
				{
					label: 'Dependencies',
					value: 'dependencies',
					description:
						'Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies.'
				},
				{
					label: 'Previews',
					value: 'previews',
					description: 'Previews are jobs that have been started in the editor as "Tests"'
				},
				{
					label: 'Sync',
					value: 'deploymentcallbacks',
					description:
						'Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings'
				}
			],
			label: 'Job kinds',
			description: 'Filter by job category'
		},
		status: {
			type: 'oneof',
			options: [
				{ label: 'All (default)', value: 'all' },
				{ label: 'Running', value: 'running' },
				{ label: 'Success', value: 'success' },
				{ label: 'Failure', value: 'failure' }
			],
			label: 'Status',
			description: 'Filter by job execution status'
		},
		show_skipped: {
			type: 'boolean',
			label: 'Show skipped',
			description: 'Include skipped flow steps'
		},
		job_trigger_kind: {
			type: 'oneof',
			label: 'Trigger kind',
			options: jobTriggerKinds.map((value) => ({
				label: triggerDisplayNamesMap[value],
				value
			})),
			description: 'Filter by how the job was triggered'
		},
		arg: {
			type: 'string',
			label: 'Args',
			description: 'Filter by job arguments (JSON format)'
		},
		result: {
			type: 'string',
			label: 'Result',
			description: 'Filter by job result (JSON format)'
		}
	} as const
}

export type RunsFilterSearchbarSchema = ReturnType<typeof buildRunsFilterSearchbarSchema>
