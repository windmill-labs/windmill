import type { PipelineTriggerDraftKind } from './types'

export const DRAFTABLE_TRIGGER_KINDS: readonly PipelineTriggerDraftKind[] = [
	'schedule',
	'email',
	'kafka',
	'mqtt',
	'amqp',
	'nats',
	'postgres',
	'sqs',
	'gcp'
]

export function isDraftableTriggerKind(kind: string): kind is PipelineTriggerDraftKind {
	return (DRAFTABLE_TRIGGER_KINDS as readonly string[]).includes(kind)
}

/** Drafts are keyed by kind too: two kinds may share a trigger path. */
export function triggerDraftKey(kind: string, path: string): string {
	return `${kind}:${path}`
}

export function defaultTriggerPath(scriptPath: string, kind: PipelineTriggerDraftKind): string {
	return `${scriptPath}_${kind}`
}
