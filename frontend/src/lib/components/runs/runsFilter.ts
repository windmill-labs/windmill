import type { JobTriggerKind } from '$lib/gen'
import {
	Braces,
	Calendar,
	Clock,
	ServerCog,
	FileCode,
	FolderIcon,
	Hash,
	ListFilter,
	Lock,
	Tag,
	UserIcon,
	Zap,
	CirclePlayIcon
} from 'lucide-svelte'
import { triggerDisplayNamesMap } from '../triggers/utils'
import type { FilterInstanceRec, FilterSchemaRec } from '../FilterSearchbar.svelte'
import { runsTimeframes } from './TimeframeSelect.svelte'

export function buildRunsFilterSearchbarSchema({
	paths,
	usernames,
	folders,
	jobTriggerKinds,
	isSuperAdmin,
	isAdminsWorkspace
}: {
	paths: string[]
	usernames: string[]
	folders: string[]
	jobTriggerKinds: JobTriggerKind[]
	isSuperAdmin: boolean
	isAdminsWorkspace: boolean
}) {
	return {
		_default_: {
			type: 'string' as const,
			hidden: true
		},
		min_ts: {
			type: 'date' as const,
			label: 'From',
			icon: Calendar,
			description: 'Only include jobs that completed after this date',
			mode: 'start',
			otherField: 'max_ts'
		},
		max_ts: {
			type: 'date' as const,
			label: 'To',
			icon: Calendar,
			description: 'Only include jobs that completed before this date',
			mode: 'end',
			otherField: 'min_ts'
		},
		timeframe: {
			type: 'oneof' as const,
			label: 'Timeframe',
			icon: Calendar,
			description: 'Predefined timeframes',
			options: runsTimeframes.map((tf) => ({ label: tf.label, value: tf.label }))
		},
		path: {
			type: 'oneof' as const,
			options: paths.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: true,
			allowMultiple: true,
			label: 'Path',
			icon: FileCode,
			description: 'Filter by script or flow path'
		},
		user: {
			type: 'oneof' as const,
			options: usernames.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: true,
			allowMultiple: true,
			label: 'User',
			icon: UserIcon,
			description: 'Filter by user who created the job'
		},
		folder: {
			type: 'oneof' as const,
			options: folders.map((s) => ({ label: s, value: s })),
			allowCustomValue: true,
			allowNegative: true,
			allowMultiple: true,
			label: 'Folder',
			icon: FolderIcon,
			description: 'Filter by folder containing the script or flow'
		},
		label: {
			type: 'string' as const,
			allowMultiple: true,
			label: 'Label',
			icon: Tag,
			description: 'Filter by custom label attached to jobs'
		},
		tag: {
			type: 'string' as const,
			allowMultiple: true,
			label: 'Tag',
			icon: Hash,
			description: 'Filter by worker tag'
		},
		worker: {
			type: 'string' as const,
			allowMultiple: true,
			label: 'Worker',
			icon: ServerCog,
			description: 'Filter by specific worker instance'
		},
		schedule_path: {
			type: 'string' as const,
			label: 'Schedule path',
			icon: Clock,
			description: 'Filter by schedule that triggered the job'
		},
		concurrency_key: {
			type: 'string' as const,
			label: 'Concurrency key',
			icon: Lock,
			description: 'Filter by concurrency limit key'
		},
		job_kinds: {
			type: 'oneof' as const,
			options: [
				{ label: 'All', value: 'all' as const },
				{
					label: 'Runs (default)',
					value: 'runs' as const,
					description:
						'Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they start), they have been triggered through the UI, a schedule or webhook'
				},
				{
					label: 'Dependencies',
					value: 'dependencies' as const,
					description:
						'Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies.'
				},
				{
					label: 'Previews',
					value: 'previews' as const,
					description: 'Previews are jobs that have been started in the editor as "Tests"'
				},
				{
					label: 'Sync',
					value: 'deploymentcallbacks' as const,
					description:
						'Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings'
				}
			],
			label: 'Job kinds',
			icon: ListFilter,
			description: 'Filter by job category'
		},
		status: {
			type: 'oneof' as const,
			options: [
				{ label: 'All (default)', value: 'all' as const },
				{ label: 'Running', value: 'running' as const },
				{ label: 'Success', value: 'success' as const },
				{ label: 'Failure', value: 'failure' as const },
				{ label: 'Waiting', value: 'waiting' as const },
				{ label: 'Suspended', value: 'suspended' as const }
			],
			label: 'Status',
			icon: CirclePlayIcon,
			description: 'Filter by job execution status'
		},
		show_skipped: {
			type: 'boolean' as const,
			label: 'Show skipped',
			description: 'Include skipped flow steps'
		},
		job_trigger_kind: {
			type: 'oneof' as const,
			label: 'Trigger kind',
			icon: Zap,
			options: jobTriggerKinds.map((value) => ({
				label: triggerDisplayNamesMap[value],
				value
			})),
			allowNegative: true,
			allowMultiple: true,
			description: 'Filter by how the job was triggered'
		},
		arg: {
			type: 'string' as const,
			format: 'json' as const,
			label: 'Args',
			icon: Braces,
			description: 'Filter by job arguments (JSON format)'
		},
		result: {
			type: 'string' as const,
			format: 'json' as const,
			label: 'Result',
			icon: Braces,
			description: 'Filter by job result (JSON format)'
		},
		show_future_jobs: {
			type: 'boolean' as const,
			label: 'Show future jobs (Default: true)',
			description: 'Include jobs that are planned later'
		},
		...(isSuperAdmin &&
			isAdminsWorkspace && {
				all_workspaces: {
					type: 'boolean' as const,
					label: 'All workspaces',
					description: 'Show jobs of all workspaces (superadmin only)'
				}
			})
	} satisfies FilterSchemaRec
}

export type RunsFilterSearchbarSchema = ReturnType<typeof buildRunsFilterSearchbarSchema>
export type RunsFilterInstance = FilterInstanceRec<RunsFilterSearchbarSchema>

export function allowWildcards(filters: Partial<RunsFilterInstance> | undefined) {
	return (
		filters?.label?.includes('*') ||
		filters?.worker?.includes('*') ||
		filters?.tag?.includes('*') ||
		false
	)
}

export const buildRunsFilterPresets = ({
	isSuperadmin,
	isAdminsWorkspace
}: {
	isSuperadmin: boolean
	isAdminsWorkspace: boolean
}) => [
	{ name: 'Hide schedules', value: 'job_trigger_kind:\\ !schedule' },
	{ name: 'Hide future jobs', value: 'show_future_jobs:\\ false' },
	{ name: 'Show skipped', value: 'show_skipped:\\ true' },
	...(isSuperadmin && isAdminsWorkspace
		? [{ name: 'All workspaces', value: 'all_workspaces:\\ true' }]
		: [])
]
