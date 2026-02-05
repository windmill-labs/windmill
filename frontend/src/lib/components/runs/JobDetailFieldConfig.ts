import type { Job } from '$lib/gen'
import { triggerIconMap } from '$lib/components/triggers/utils'
import { formatMemory } from '$lib/utils'
import { Calendar, Bot } from 'lucide-svelte'
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
	| 'permissioned_as'
	| 'memory_peak'
	| 'run_id'
	| 'trigger_info'
	| 'parent_job'
	| 'schedule_path'
	| 'script_hash'
	| 'script_path'
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
	label: string
	getValue: (job: Job) => string | null
	getHref?: (job: Job, workspaceId: string) => string | null
}

/**
 * Configuration for field presence per job category
 */
export type FieldPresenceConfig = Record<JobCategory, Record<JobField, boolean>>

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
		label: 'Received',
		getValue: (job) => job.created_at || null
	},

	started_at: {
		field: 'started_at',
		label: 'Started',
		getValue: (job) => ('started_at' in job ? job.started_at : null) || null
	},

	created_by: {
		field: 'created_by',
		label: 'By',
		getValue: (job) => job.created_by || 'unknown'
	},

	permissioned_as: {
		field: 'permissioned_as',
		label: 'Permissioned as',
		getValue: (job) => job.permissioned_as || null
	},

	memory_peak: {
		field: 'memory_peak',
		label: 'Mem peak',
		getValue: (job) => {
			if (!job.mem_peak) return null
			const formatted = formatMemory(job.mem_peak, true)
			return `${formatted.display}|${formatted.tooltip}`
		}
	},

	run_id: {
		field: 'run_id',
		label: 'Run ID',
		getValue: (job) => job.id
	},

	trigger_info: {
		field: 'trigger_info',
		label: 'Trigger',
		getValue: (job) => {
			const triggerInfo = getTriggerInfo(job)
			return triggerInfo ? triggerInfo.type : null
		}
	},

	parent_job: {
		field: 'parent_job',
		label: 'Parent',
		getValue: (job) => job.parent_job || null,
		getHref: (job, workspaceId) =>
			job.parent_job ? `/run/${job.parent_job}?workspace=${workspaceId}` : null
	},

	schedule_path: {
		field: 'schedule_path',
		label: 'Schedule',
		getValue: (job) => job.schedule_path || null
	},

	script_hash: {
		field: 'script_hash',
		label: 'Hash',
		getValue: (job) => job.script_hash?.toString() || null,
		getHref: (job, workspaceId) =>
			job.script_hash && job.job_kind === 'script'
				? `/scripts/get/${job.script_hash}?workspace=${workspaceId}`
				: null
	},

	script_path: {
		field: 'script_path',
		label: 'Path',
		getValue: (job) => job.script_path || null,
		getHref: (job, workspaceId) => {
			if (!job.script_path) return null
			const stem = job.job_kind === 'script' ? 'scripts' : 'flows'
			const isScript = job.job_kind === 'script'
			return `/${stem}/get/${isScript ? job.script_hash : job.script_path}`
		}
	},

	worker: {
		field: 'worker',
		label: 'Worker',
		getValue: (job) => job.worker || null
	},

	language: {
		field: 'language',
		label: 'Language',
		getValue: (job) => job.language || null
	},

	step_info: {
		field: 'step_info',
		label: 'Step',
		getValue: (job) => {
			if (job.is_flow_step && 'flow_step_id' in job) {
				return (job as any).flow_step_id
			}
			return null
		}
	},

	flow_status: {
		field: 'flow_status',
		label: 'Flow Status',
		getValue: (job) => {
			if (job.flow_status?.step !== undefined) {
				return `Step ${job.flow_status.step + 1}`
			}
			return null
		}
	},

	duration: {
		field: 'duration',
		label: 'Duration',
		getValue: (job) => {
			if ('duration_ms' in job && job.duration_ms) {
				const ms = job.duration_ms
				if (ms < 1000) return `${ms}ms`
				if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
				return `${(ms / 60000).toFixed(1)}m`
			}
			return null
		}
	},

	priority: {
		field: 'priority',
		label: 'Priority',
		getValue: (job) => job.priority?.toString() || null
	},

	concurrency_key: {
		field: 'concurrency_key',
		label: 'Concurrency',
		getValue: () => null // This will be provided separately as a prop
	}
}

/**
 * Field presence configuration per job category
 * Defines which fields should be shown for each job type
 */
export const categoryFieldPresence: FieldPresenceConfig = {
	script: {
		created_at: true,
		created_by: true,
		permissioned_as: false, // Will be conditionally shown
		worker: true,
		run_id: true,
		started_at: true,
		memory_peak: true,
		script_hash: true,
		language: true,
		duration: false,
		// Optional fields
		trigger_info: false,
		parent_job: false,
		schedule_path: false,
		script_path: false,
		step_info: false,
		flow_status: false,
		priority: false,
		concurrency_key: false
	},
	flow: {
		created_at: true,
		started_at: true,
		created_by: true,
		permissioned_as: false, // Will be conditionally shown
		worker: true,
		run_id: true,
		script_path: true,
		flow_status: false,
		duration: false,
		// Optional fields
		memory_peak: false,
		trigger_info: false,
		parent_job: false,
		schedule_path: false,
		script_hash: false,
		language: false,
		step_info: false,
		priority: false,
		concurrency_key: false
	},
	dependencies: {
		created_at: true,
		created_by: true,
		permissioned_as: false, // Will be conditionally shown
		worker: true,
		run_id: true,
		started_at: true,
		duration: false,
		// Optional fields
		memory_peak: false,
		trigger_info: false,
		parent_job: false,
		schedule_path: false,
		script_hash: false,
		script_path: false,
		language: false,
		step_info: false,
		flow_status: false,
		priority: false,
		concurrency_key: false
	},
	flow_node: {
		created_at: true,
		created_by: true,
		permissioned_as: false, // Will be conditionally shown
		worker: true,
		run_id: true,
		started_at: true,
		step_info: false,
		script_hash: true,
		memory_peak: true,
		parent_job: true,
		// Optional fields
		trigger_info: false,
		schedule_path: false,
		script_path: false,
		language: false,
		flow_status: false,
		duration: false,
		priority: false,
		concurrency_key: false
	},
	ai_agent: {
		created_at: true,
		created_by: true,
		permissioned_as: false, // Will be conditionally shown
		worker: true,
		run_id: true,
		started_at: true,
		duration: false,
		memory_peak: true,
		// Optional fields
		trigger_info: false,
		parent_job: false,
		schedule_path: false,
		script_hash: false,
		script_path: false,
		language: false,
		step_info: false,
		flow_status: false,
		priority: false,
		concurrency_key: false
	},
	system: {
		created_at: true,
		created_by: true,
		permissioned_as: false, // Will be conditionally shown
		worker: true,
		run_id: true,
		started_at: true,
		duration: false,
		// Optional fields
		memory_peak: false,
		trigger_info: false,
		parent_job: false,
		schedule_path: false,
		script_hash: false,
		script_path: false,
		language: false,
		step_info: false,
		flow_status: false,
		priority: false,
		concurrency_key: false
	},
	trigger_job: {
		created_at: true,
		created_by: true,
		permissioned_as: false, // Will be conditionally shown
		worker: true,
		run_id: true,
		started_at: true,
		parent_job: true,
		schedule_path: true,
		trigger_info: true,
		// Optional fields
		memory_peak: false,
		script_hash: false,
		script_path: false,
		language: false,
		step_info: false,
		flow_status: false,
		duration: false,
		priority: false,
		concurrency_key: false
	}
}

/**
 * Gets relevant fields for a job based on its category and characteristics
 */
export function getRelevantFields(job: Job): FieldConfig[] {
	const category = getJobCategory(job)
	const fieldsPresence = categoryFieldPresence[category]

	// Conditionally show permissioned_as field when different from created_by
	const shouldShowPermissionedAs =
		job.permissioned_as !== `u/${job.created_by}` && job.permissioned_as !== job.created_by

	// Define the field ordering: created_by, permissioned_as, started_at, worker, then others
	const fieldOrder: JobField[] = [
		'created_at',
		'started_at',
		'created_by',
		'permissioned_as',
		'worker',
		'run_id',
		'memory_peak',
		'script_hash',
		'script_path',
		'language',
		'duration',
		'flow_status',
		'step_info',
		'parent_job',
		'schedule_path',
		'trigger_info',
		'priority',
		'concurrency_key'
	]

	// Return fields in the specified order if they are present for this category
	return fieldOrder
		.filter((fieldName) => {
			if (fieldName === 'permissioned_as') {
				return shouldShowPermissionedAs
			}
			if (fieldName === 'parent_job') {
				// Always show parent_job when it exists, regardless of category configuration
				return job.parent_job !== null && job.parent_job !== undefined
			}
			return fieldsPresence[fieldName]
		})
		.map((fieldName) => fieldConfigs[fieldName])
		.filter((config) => config) // Safety check
}
