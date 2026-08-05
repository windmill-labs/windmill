import { getContext, setContext } from 'svelte'
import { logFeatureUsage } from '$lib/utils/featureUsage'
import { randomUUID } from '$lib/utils/uuid'
import type { StepSettingKey, StepSettingView } from './flowStepSettings'
import type { FlowPanelMode, FlowPanelPreference } from './panelPlacement'

// Anonymous counters for the flow editor's step/loop/branch panels. Same rules as every
// other `logFeatureUsage` caller: aggregated counts only, never a path, an expression or a
// step id — the keys below are a closed vocabulary and the entity id is an opaque random id.

export const FLOW_EDITOR_FEATURE = 'flow_editor'

export type FlowEditorTelemetryKind =
	/** A step's panel came on screen. Key: the placement it opened in. */
	| 'panel_open'
	/** That panel went away. Key: `<placement>:<duration bucket>`. */
	| 'panel_dwell'
	/** The placement preference was changed by hand. Key: `<preference>:from_<placement>`. */
	| 'placement'
	/** A setting was configured or cleared. Key: `<StepSettingKey>:on|off`. */
	| 'setting'
	/** A setting's summary reads as invalid. Key: the `StepSettingKey`. */
	| 'setting_invalid'
	/** Prop-picker connect lifecycle. Key: `<input|expression>:<open|insert|abandon>`. */
	| 'connect'
	/** AI suggestions. Key: `<step_input|iterator>:<generate|accept_click|accept_tab|discard>`. */
	| 'ai_input'
	/** Step header actions. Key: `menu_open` | `save_to_workspace`. */
	| 'header_action'

/**
 * Kinds carrying the editor-session entity id, so the stats job's per-entity median/p90
 * answers "how much of this per editing session". The rest stay plain counters: an entity
 * id costs a row per session per day, which is only worth paying where the spread is the
 * question.
 */
const PER_SESSION_KINDS: ReadonlySet<FlowEditorTelemetryKind> = new Set(['panel_open', 'setting'])

export interface FlowEditorTelemetry {
	log(kind: FlowEditorTelemetryKind, key: string): void
}

const CONTEXT_KEY = 'flowEditorTelemetry'

/** These components also mount outside a flow editor, where there is nothing to measure. */
const NOOP: FlowEditorTelemetry = { log: () => {} }

/**
 * Opens an editing session and publishes its logger to the panel's components. One session
 * per editor mount: it is the unit the per-entity distributions are read against.
 */
export function setFlowEditorTelemetry(): FlowEditorTelemetry {
	const sessionId = randomUUID()
	const telemetry: FlowEditorTelemetry = {
		log(kind, key) {
			logFeatureUsage(FLOW_EDITOR_FEATURE, kind, {
				key,
				entityId: PER_SESSION_KINDS.has(kind) ? sessionId : undefined
			})
		}
	}
	setContext<FlowEditorTelemetry>(CONTEXT_KEY, telemetry)
	return telemetry
}

export function useFlowEditorTelemetry(): FlowEditorTelemetry {
	return getContext<FlowEditorTelemetry | undefined>(CONTEXT_KEY) ?? NOOP
}

const DWELL_BUCKETS: readonly (readonly [number, string])[] = [
	[5_000, '0-5s'],
	[30_000, '5-30s'],
	[120_000, '30-120s']
]

/**
 * Buckets a duration into a fixed set of keys rather than logging the seconds against a
 * per-visit entity id: a histogram of four keys costs four rows a day, where per-visit ids
 * would cost one per panel opened.
 */
export function dwellBucket(ms: number): string {
	for (const [limit, label] of DWELL_BUCKETS) {
		if (ms < limit) return label
	}
	return '120s+'
}

type Log = (kind: FlowEditorTelemetryKind, key: string) => void

/**
 * Pairs each panel visit with the dwell it ended in. Deliberately free of runes so the
 * transitions can be tested without a component — the failure a refactor introduces
 * silently is a second `panel_open` for a visit that never ended.
 */
export function createPanelVisitTracker(log: Log, now: () => number = () => Date.now()) {
	let current: { target: string; mode: FlowPanelMode; at: number } | undefined

	function close() {
		if (!current) return
		log('panel_dwell', `${current.mode}:${dwellBucket(now() - current.at)}`)
		current = undefined
	}

	return {
		/**
		 * `target` is only ever compared, never logged. Moving the same step between
		 * placements ends the visit and starts another: the dwell belongs to a placement,
		 * so it cannot span two.
		 */
		visit(target: string | undefined, mode: FlowPanelMode) {
			if (current?.target === target && current?.mode === mode) return
			close()
			if (target === undefined) return
			current = { target, mode, at: now() }
			log('panel_open', mode)
		},
		end: close
	}
}

/**
 * Turns the settings a step currently has into the changes the user just made, off the same
 * `describeStepSettings` view the graph badges read — so the telemetry vocabulary cannot
 * drift from the one on screen.
 */
export function createSettingsChangeTracker(log: Log) {
	let moduleId: string | undefined
	let configured = new Set<StepSettingKey>()
	let invalid = new Set<StepSettingKey>()

	return {
		observe(nextModuleId: string, views: StepSettingView[]) {
			const nextConfigured = new Set(views.filter((v) => v.configured).map((v) => v.key))
			const nextInvalid = new Set(
				views.filter((v) => v.summary.state === 'invalid').map((v) => v.key)
			)
			// Another step is another baseline. Without this, selecting a configured step reads
			// as the user having just switched all of its settings on.
			if (nextModuleId !== moduleId) {
				moduleId = nextModuleId
			} else {
				for (const key of nextConfigured) {
					if (!configured.has(key)) log('setting', `${key}:on`)
				}
				for (const key of configured) {
					if (!nextConfigured.has(key)) log('setting', `${key}:off`)
				}
				for (const key of nextInvalid) {
					if (!invalid.has(key)) log('setting_invalid', key)
				}
			}
			configured = nextConfigured
			invalid = nextInvalid
		}
	}
}

/** Reads as "what the user asked for, given where the panel already was". */
export function placementKey(preference: FlowPanelPreference, mode: FlowPanelMode): string {
	return `${preference}:from_${mode}`
}
