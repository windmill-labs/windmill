import type { Job } from '$lib/gen'
import { triggerIconMap } from '$lib/components/triggers/utils'
import {
	Clock,
	MemoryStick,
	Calendar,
	Bot,
	User,
	Code,
	IdCard,
	Tag,
	Hash,
	HardHat,
	Webhook,
	FileText,
	GitBranch,
	PlayCircle,
	Layers
} from 'lucide-svelte'
import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

/**
 * Categories of job types for field configuration
 */
export type JobCategory =
	| 'script'
	| 'flow'
	| 'dependencies'
	| 'flow_node'
	| 'ai_agent'
	| 'system'
	| 'trigger_job'

/**
 * Field types that can be displayed in the job header
 */
export type JobField =
	| 'created_at'
	| 'started_at'
	| 'created_by'
	| 'memory_peak'
	| 'run_id'
	| 'trigger_info'
	| 'parent_job'
	| 'schedule_path'
	| 'script_hash'
	| 'script_path'
	| 'tag'
	| 'worker'
	| 'language'
	| 'step_info'
	| 'flow_status'
	| 'duration'
	| 'priority'
	| 'concurrency_key'

/**
 * Configuration for how a field should be displayed
 */
export interface FieldConfig {
	field: JobField
	icon: any
	label: string
	getValue: (job: Job) => string | null
	getHref?: (job: Job, workspaceId: string) => string | null
	isRelevant: (job: Job) => boolean
	priority: number // Higher numbers = more important, shown first
}

/**
 * Categorizes a job based on its job_kind
 */
export function getJobCategory(job: Job): JobCategory {
	switch (job.job_kind) {
		case 'script':
		case 'preview':
		case 'script_hub':
			return 'script'

		case 'flow':
		case 'flowpreview':
		case 'singlestepflow':
			return 'flow'

		case 'dependencies':
		case 'flowdependencies':
		case 'appdependencies':
			return 'dependencies'

		case 'flownode':
		case 'flowscript':
		case 'appscript':
			return 'flow_node'

		case 'aiagent':
			return 'ai_agent'

		case 'identity':
		case 'deploymentcallback':
			return 'system'

		default:
			// Check if it was triggered by something other than manual/schedule
			if (job.parent_job || job.schedule_path) {
				return 'trigger_job'
			}
			return 'script' // Default fallback
	}
}

/**
 * Gets trigger information from a job
 */
export function getTriggerInfo(job: Job): { type: string; icon: any; detail?: string } | null {
	if (job.schedule_path) {
		return {
			type: 'Schedule',
			icon: Calendar,
			detail: job.schedule_path
		}
	}

	// Check for trigger type from job trigger_kind if available
	if ('trigger_kind' in job) {
		const triggerKind = (job as any).trigger_kind
		if (triggerKind && triggerIconMap[triggerKind]) {
			return {
				type: getTriggerDisplayName(triggerKind),
				icon: triggerIconMap[triggerKind],
				detail: triggerKind
			}
		}
	}

	// Infer from job properties
	if (job.parent_job) {
		if (job.is_flow_step) {
			return {
				type: 'Flow Step',
				icon: BarsStaggered
			}
		} else {
			return {
				type: 'Triggered',
				icon: Bot
			}
		}
	}

	return null
}

/**
 * Gets human-readable display name for trigger kinds
 */
function getTriggerDisplayName(triggerKind: string): string {
	const displayNames: Record<string, string> = {
		webhook: 'Webhook',
		http: 'HTTP',
		websocket: 'WebSocket',
		postgres: 'PostgreSQL',
		kafka: 'Kafka',
		nats: 'NATS',
		mqtt: 'MQTT',
		sqs: 'SQS',
		gcp: 'GCP Pub/Sub',
		email: 'Email',
		schedule: 'Schedule',
		app: 'App',
		ui: 'UI'
	}
	return displayNames[triggerKind] || triggerKind.toUpperCase()
}

/**
 * Field configurations for all possible fields
 */
export const fieldConfigs: Record<JobField, FieldConfig> = {
	created_at: {
		field: 'created_at',
		icon: Clock,
		label: 'Received',
		getValue: (job) => job.created_at || null,
		isRelevant: () => true,
		priority: 9
	},

	started_at: {
		field: 'started_at',
		icon: PlayCircle,
		label: 'Started',
		getValue: (job) => ('started_at' in job ? job.started_at : null) || null,
		isRelevant: (job) => 'started_at' in job && job.started_at != null,
		priority: 8
	},

	created_by: {
		field: 'created_by',
		icon: User,
		label: 'By',
		getValue: (job) => job.created_by || 'unknown',
		isRelevant: () => true,
		priority: 7
	},

	memory_peak: {
		field: 'memory_peak',
		icon: MemoryStick,
		label: 'Mem peak',
		getValue: (job) => (job.mem_peak ? `${(job.mem_peak / 1024).toPrecision(5)}MB` : null),
		isRelevant: (job) => {
			const category = getJobCategory(job)
			return category === 'script' || category === 'flow_node'
		},
		priority: 4
	},

	run_id: {
		field: 'run_id',
		icon: IdCard,
		label: 'Run ID',
		getValue: (job) => job.id,
		isRelevant: () => true,
		priority: 6
	},

	trigger_info: {
		field: 'trigger_info',
		icon: Webhook, // Default, will be overridden by getTriggerInfo
		label: 'Trigger',
		getValue: (job) => {
			const triggerInfo = getTriggerInfo(job)
			return triggerInfo ? triggerInfo.type : null
		},
		isRelevant: (job) => getTriggerInfo(job) !== null,
		priority: 5
	},

	parent_job: {
		field: 'parent_job',
		icon: GitBranch,
		label: 'Parent',
		getValue: (job) => job.parent_job || null,
		getHref: (job, workspaceId) =>
			job.parent_job ? `/run/${job.parent_job}?workspace=${workspaceId}` : null,
		isRelevant: (job) => !!job.parent_job,
		priority: 5
	},

	schedule_path: {
		field: 'schedule_path',
		icon: Calendar,
		label: 'Schedule',
		getValue: (job) => job.schedule_path || null,
		isRelevant: (job) => !!job.schedule_path,
		priority: 5
	},

	script_hash: {
		field: 'script_hash',
		icon: Hash,
		label: 'Hash',
		getValue: (job) => job.script_hash?.toString() || null,
		getHref: (job, workspaceId) =>
			job.script_hash && job.job_kind === 'script'
				? `/scripts/get/${job.script_hash}?workspace=${workspaceId}`
				: null,
		isRelevant: (job) => {
			const category = getJobCategory(job)
			return (
				(category === 'script' || category === 'flow_node') &&
				!!job.script_hash &&
				job.job_kind !== 'aiagent'
			)
		},
		priority: 3
	},

	script_path: {
		field: 'script_path',
		icon: Code,
		label: 'Path',
		getValue: (job) => job.script_path || null,
		getHref: (job, workspaceId) => {
			if (!job.script_path) return null
			const stem = job.job_kind === 'script' ? 'scripts' : 'flows'
			const isScript = job.job_kind === 'script'
			return `/${stem}/get/${isScript ? job.script_hash : job.script_path}`
		},
		isRelevant: (job) => !!job.script_path,
		priority: 6
	},

	tag: {
		field: 'tag',
		icon: Tag,
		label: 'Tag',
		getValue: (job) => job.tag || null,
		isRelevant: (job) => {
			const category = getJobCategory(job)
			return (category === 'script' || category === 'flow') && !!job.tag
		},
		priority: 2
	},

	worker: {
		field: 'worker',
		icon: HardHat,
		label: 'Worker',
		getValue: (job) => job.worker || null,
		isRelevant: (job) => !!job.worker,
		priority: 3
	},

	language: {
		field: 'language',
		icon: Code,
		label: 'Language',
		getValue: (job) => job.language || null,
		isRelevant: (job) => {
			const category = getJobCategory(job)
			return (category === 'script' || category === 'flow_node') && !!job.language
		},
		priority: 4
	},

	step_info: {
		field: 'step_info',
		icon: Layers,
		label: 'Step',
		getValue: (job) => {
			if (job.is_flow_step && 'flow_step_id' in job) {
				return (job as any).flow_step_id
			}
			return null
		},
		isRelevant: (job) => {
			const category = getJobCategory(job)
			return category === 'flow_node' && job.is_flow_step
		},
		priority: 6
	},

	flow_status: {
		field: 'flow_status',
		icon: BarsStaggered,
		label: 'Flow Status',
		getValue: (job) => {
			if (job.flow_status?.step !== undefined) {
				return `Step ${job.flow_status.step + 1}`
			}
			return null
		},
		isRelevant: (job) => {
			const category = getJobCategory(job)
			return category === 'flow' && !!job.flow_status
		},
		priority: 5
	},

	duration: {
		field: 'duration',
		icon: Clock,
		label: 'Duration',
		getValue: (job) => {
			if ('duration_ms' in job && job.duration_ms) {
				const ms = job.duration_ms
				if (ms < 1000) return `${ms}ms`
				if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
				return `${(ms / 60000).toFixed(1)}m`
			}
			return null
		},
		isRelevant: (job) => 'duration_ms' in job && !!job.duration_ms,
		priority: 4
	},

	priority: {
		field: 'priority',
		icon: FileText,
		label: 'Priority',
		getValue: (job) => job.priority?.toString() || null,
		isRelevant: (job) => job.priority != null && job.priority > 0,
		priority: 2
	},

	concurrency_key: {
		field: 'concurrency_key',
		icon: Layers,
		label: 'Concurrency',
		getValue: () => null, // This will be provided separately as a prop
		isRelevant: () => false, // Handled separately
		priority: 2
	}
}

/**
 * Gets relevant fields for a job based on its category and characteristics
 */
export function getRelevantFields(job: Job): FieldConfig[] {
	const category = getJobCategory(job)
	const allFields = Object.values(fieldConfigs)

	// Filter fields based on relevance for this job
	const relevantFields = allFields.filter((config) => config.isRelevant(job))

	// Apply category-specific prioritization
	let prioritizedFields = relevantFields.sort((a, b) => b.priority - a.priority)

	// Category-specific field limits and ordering
	switch (category) {
		case 'script':
			// Show up to 6 fields for scripts
			prioritizedFields = prioritizedFields.slice(0, 6)
			break

		case 'flow':
			// For flows, prioritize flow-specific fields
			prioritizedFields = prioritizedFields.slice(0, 6)
			break

		case 'dependencies':
			// Dependencies jobs need minimal fields
			prioritizedFields = prioritizedFields
				.filter((f) => ['created_at', 'created_by', 'worker', 'duration'].includes(f.field))
				.slice(0, 4)
			break

		case 'flow_node':
			// Flow nodes should show step info prominently
			prioritizedFields = prioritizedFields.slice(0, 5)
			break

		case 'ai_agent':
			// AI agents have specific relevant fields
			prioritizedFields = prioritizedFields
				.filter((f) => !['script_hash', 'tag'].includes(f.field))
				.slice(0, 5)
			break

		case 'system':
			// System jobs need minimal display
			prioritizedFields = prioritizedFields
				.filter((f) => ['created_at', 'created_by', 'worker', 'duration'].includes(f.field))
				.slice(0, 4)
			break

		case 'trigger_job':
			// Triggered jobs should emphasize trigger info
			prioritizedFields = prioritizedFields.slice(0, 6)
			break
	}

	return prioritizedFields
}
