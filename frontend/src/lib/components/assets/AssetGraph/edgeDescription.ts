import { describeSchedule } from '$lib/utils/describeCron'
import { NO_ASSET_NODE_ID } from './assetsOnlyView'

/** What a hovered edge says: "`subject` `phrase` `object`", then a note. */
export type EdgeDescription = { subject: string; phrase: string; object?: string; note?: string }

/** The edge fields the description reads (the canvas model's edge). */
export type DescribedEdge = {
	source: string
	target: string
	kind: string
	unsaved?: boolean
	muted?: boolean
	missing?: boolean
	via?: string[]
	macro_names?: string[]
	via_use?: boolean
}

// What starts a script, per trigger kind: "runs on <event>".
const TRIGGER_EVENT: Record<string, string> = {
	kafka: 'Kafka message',
	mqtt: 'MQTT message',
	amqp: 'AMQP message',
	nats: 'NATS message',
	sqs: 'SQS message',
	gcp: 'Pub/Sub message',
	email: 'incoming email',
	postgres: 'Postgres event',
	webhook: 'webhook call',
	data_upload: 'data upload'
}

/**
 * The relationship an edge draws, as a sentence about its two ends. `nameOf`
 * names a node, `dataOf` gives its data (a trigger's kind and cadence).
 */
export function describeEdge(
	e: DescribedEdge,
	nameOf: (id: string) => string,
	dataOf: (id: string) => any
): EdgeDescription | undefined {
	const src = nameOf(e.source)
	const tgt = nameOf(e.target)
	const draft = e.unsaved ? 'Not deployed yet.' : undefined
	switch (e.kind) {
		case 'lineage-write':
			return { subject: src, phrase: 'writes to', object: tgt, note: draft }
		case 'lineage-read':
			return {
				subject: tgt,
				phrase: 'reads from',
				object: src,
				note: e.muted ? 'A write to it does not run the script (muted).' : draft
			}
		case 'trigger-asset':
			return { subject: tgt, phrase: 'runs after each write to', object: src, note: draft }
		case 'trigger-native': {
			const t = dataOf(e.source) ?? {}
			if (e.missing) {
				return {
					subject: tgt,
					phrase: `declares \`// on ${t.kind}\`, but no trigger of that kind targets it`
				}
			}
			const phrase =
				t.kind === 'schedule' && t.schedule
					? `runs on schedule: ${describeSchedule(t.schedule, t.timezone) ?? t.schedule}`
					: `runs on ${TRIGGER_EVENT[t.kind] ?? `${t.kind} trigger`}`
			return { subject: tgt, phrase, note: t.draft ? 'Draft trigger, created on save.' : draft }
		}
		case 'asset-flow': {
			const via = (e.via ?? []).map(nameOf).join(', ')
			// The scripts that build nothing only read: name them, not the node.
			if (e.target === NO_ASSET_NODE_ID) {
				return { subject: src, phrase: 'is read by', object: via || tgt }
			}
			return {
				subject: tgt,
				phrase: 'is built from',
				object: src,
				note: via ? `By ${via}.` : undefined
			}
		}
		case 'dbt-ref':
			return { subject: tgt, phrase: 'references', object: src, note: 'dbt ref().' }
		case 'macro':
			return {
				subject: tgt,
				phrase: e.via_use ? 'uses the macro library' : 'uses macros from',
				object: src,
				note: e.macro_names?.length ? e.macro_names.join(', ') : undefined
			}
		case 'test-dependency':
			return {
				subject: src,
				phrase: 'must run before',
				object: tgt,
				note: 'Its data test reads what the first one builds.'
			}
		case 'data-test':
			return { subject: tgt, phrase: 'tests', object: src }
		default:
			return undefined
	}
}
