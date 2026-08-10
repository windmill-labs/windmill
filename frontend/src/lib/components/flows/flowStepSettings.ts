import {
	ChevronsUp,
	CircleStop,
	Combine,
	Database,
	Gauge,
	Hand,
	Moon,
	RefreshCw,
	ShieldAlert,
	SkipForward,
	Timer,
	Trash2
} from 'lucide-svelte'
import type { FlowModule } from '$lib/gen'
import type { ScriptAdvancedSettingsFields } from '$lib/components/scriptSettings'
import { validateRetryConfig } from '$lib/utils'

// Single source for the per-step runtime settings: which apply, which are configured,
// and how each reads back. Graph badges, the run-settings accordion and the setting
// editors all read it here so they cannot drift. Script-level twin: `scriptSettings.ts`.

export type StepSettingKey =
	| 'skip'
	| 'early-stop'
	| 'suspend'
	| 'sleep'
	| 'retries'
	| 'error-handling'
	| 'timeout'
	| 'concurrency'
	| 'priority'
	| 'cache'
	| 'debounce'
	| 'lifetime'

export type StepSettingSummary = {
	text: string
	state: 'configured' | 'default' | 'invalid'
	/** Render the text as code (it is a user-written expression). */
	mono?: boolean
}

export type StepSettingView = {
	key: StepSettingKey
	label: string
	/** Longer wording for hover surfaces; falls back to `label`. */
	tooltip: string
	icon: any
	/** The setting's config is present on the step. Deliberately not "the runtime
	 *  would behave differently" — a configured setting can still be a no-op
	 *  (a sleep of 0), and `summary` says so rather than claiming it is off. */
	configured: boolean
	summary: StepSettingSummary
}

/** A step whose script polls an external system and returns the new items. */
export function isTriggerStep(module: FlowModule | undefined): boolean {
	return (
		module?.value != undefined &&
		(module.value.type === 'script' || module.value.type === 'rawscript') &&
		module.value.is_trigger === true
	)
}

const def = (text: string): StepSettingSummary => ({ text, state: 'default' })
const inv = (text: string): StepSettingSummary => ({ text, state: 'invalid' })
const cfg = (text: string, mono = false): StepSettingSummary => ({
	text,
	state: 'configured',
	mono
})

function formatDur(s: number | undefined): string {
	if (s == null) return ''
	if (s < 60) return `${s}s`
	if (s < 3600) return `${Math.round(s / 60)} min`
	return `${Math.round(s / 3600)} h`
}

/** Describe a user-written predicate. An empty expression is still configured —
 *  the setting is on, it just has nothing to evaluate yet. */
function exprSummary(expr: string | undefined): StepSettingSummary {
	const e = expr?.trim()
	if (!e) return cfg('No expression')
	return e.length <= 24 ? cfg(e, true) : cfg('Expression set')
}

type Ctx = { referenced?: ScriptAdvancedSettingsFields }

type SettingSpec = {
	label: string
	tooltip?: (mod: FlowModule) => string
	icon: any
	applies?: (mod: FlowModule) => boolean
	configured: (mod: FlowModule, ctx: Ctx) => boolean
	summarize: (mod: FlowModule, ctx: Ctx) => StepSettingSummary
}

// A value the step itself carries counts as configured even when the runtime would
// ignore it — `summary` is what reports the no-op, and the setting editors keep their
// controls live on presence so clearing a field mid-edit can't disable it. Only values
// read off a referenced workspace script use the runtime's `> 0` test: nobody is typing
// into those, and 0 there simply means the script has no limit.
const isWorkspaceScript = (mod: FlowModule) => mod.value.type === 'script'
const inlineConcurrentLimit = (mod: FlowModule) =>
	mod.value.type === 'rawscript' ? mod.value.concurrent_limit : undefined
const effectiveCacheTtl = (mod: FlowModule, ctx: Ctx) =>
	mod.cache_ttl ?? (isWorkspaceScript(mod) ? ctx.referenced?.cache_ttl : undefined)

/** Whether an inline step carries a concurrency limit at all. The setting editor keeps its
 *  controls live on presence, so clearing the field mid-edit can't disable the input. A
 *  present-but-non-positive limit is surfaced as invalid rather than silently as "None".
 *  Strict: an emptied number input binds to `null`, which is still a value being typed. */
export function hasInlineConcurrency(mod: FlowModule): boolean {
	return inlineConcurrentLimit(mod) !== undefined
}

/** Canonical order — every surface lists settings in this sequence. */
const SPECS: { key: StepSettingKey; spec: SettingSpec }[] = [
	{
		key: 'skip',
		spec: {
			label: 'Skip if',
			icon: SkipForward,
			configured: (m) => Boolean(m.skip_if),
			summarize: (m) => (m.skip_if ? exprSummary(m.skip_if.expr) : def('Off'))
		}
	},
	{
		key: 'early-stop',
		spec: {
			label: 'Early stop / break',
			tooltip: (m) =>
				isTriggerStep(m) ? 'Stop early if there are no new events' : 'Early stop / break',
			icon: CircleStop,
			configured: (m) => m.stop_after_if != undefined || m.stop_after_all_iters_if != undefined,
			summarize: (m) => {
				// Both can be set on a sequential loop, so pick the first that carries an
				// expression instead of letting an empty one mask the other.
				const exprs = [m.stop_after_if?.expr, m.stop_after_all_iters_if?.expr].filter(
					(e) => e != undefined
				)
				if (exprs.length === 0) return def('Off')
				return exprSummary(exprs.find((e) => e?.trim()) ?? exprs[0])
			}
		}
	},
	{
		key: 'suspend',
		spec: {
			label: 'Suspend until approval/resume',
			icon: Hand,
			configured: (m) => Boolean(m.suspend),
			summarize: (m) => {
				if (!m.suspend) return def('Off')
				const n = m.suspend.required_events ?? 1
				return cfg(`${n} approval${n > 1 ? 's' : ''}`)
			}
		}
	},
	{
		key: 'sleep',
		spec: {
			label: 'Sleep',
			icon: Moon,
			configured: (m) => Boolean(m.sleep),
			summarize: (m) => {
				const s = m.sleep
				if (!s) return def('Off')
				if (s.type === 'static') {
					const v = Number(s.value)
					return Number.isFinite(v) ? cfg(`${formatDur(v)} after`) : cfg('Dynamic')
				}
				return cfg('Dynamic')
			}
		}
	},
	{
		key: 'retries',
		spec: {
			label: 'Retries',
			icon: RefreshCw,
			configured: (m) => m.retry?.constant != undefined || m.retry?.exponential != undefined,
			summarize: (m) => {
				const r = m.retry
				if (r?.constant == undefined && r?.exponential == undefined) return def('None')
				if (validateRetryConfig(r)) return { text: 'Invalid', state: 'invalid' }
				const isConstant = r?.constant != undefined
				const n = (isConstant ? r?.constant?.attempts : r?.exponential?.attempts) ?? 0
				const kind = isConstant ? 'constant' : 'exponential'
				return cfg(`${n} attempt${n === 1 ? '' : 's'}, ${kind}`)
			}
		}
	},
	{
		key: 'error-handling',
		spec: {
			label: 'Error handling',
			icon: ShieldAlert,
			configured: (m) => Boolean(m.continue_on_error),
			summarize: (m) => (m.continue_on_error ? cfg('Continue on error') : def('Off'))
		}
	},
	{
		key: 'timeout',
		spec: {
			label: 'Timeout',
			icon: Timer,
			configured: (m) => m.timeout != null,
			summarize: (m) => {
				const t = m.timeout
				if (t == null) return def('None')
				if (typeof t === 'number') return cfg(formatDur(t))
				if (t.type === 'static') {
					const v = Number(t.value)
					return Number.isFinite(v) ? cfg(formatDur(v)) : cfg('Dynamic')
				}
				return cfg('Dynamic')
			}
		}
	},
	{
		key: 'concurrency',
		spec: {
			label: 'Concurrency limit',
			icon: Gauge,
			applies: (m) => m.value.type === 'rawscript' || m.value.type === 'script',
			// Presence for the step's own limit (the user may be mid-edit, and `summary` says
			// when it is a no-op), effectiveness for the referenced script's — a remote
			// value nobody is typing into, where 0 just means the script has no limit.
			configured: (m, ctx) =>
				isWorkspaceScript(m)
					? ctx.referenced?.concurrent_limit != undefined && ctx.referenced.concurrent_limit > 0
					: hasInlineConcurrency(m),
			summarize: (m, ctx) => {
				if (isWorkspaceScript(m)) {
					const l = ctx.referenced?.concurrent_limit
					return l != undefined && l > 0 ? cfg(`Max ${l}`) : def('None')
				}
				const l = inlineConcurrentLimit(m)
				if (l === undefined) return def('None')
				if (!(l > 0)) return inv('Invalid limit')
				const key = m.value.type === 'rawscript' ? m.value.custom_concurrency_key : undefined
				return cfg(`Max ${l}${key ? ' per key' : ''}`)
			}
		}
	},
	{
		key: 'priority',
		spec: {
			label: 'Priority',
			icon: ChevronsUp,
			configured: (m) => m.priority !== undefined,
			summarize: (m) => {
				if (m.priority === undefined) return def('Off')
				// 0 is how the runtime spells "no priority".
				return m.priority > 0 ? cfg('High priority') : inv('No priority set')
			}
		}
	},
	{
		key: 'cache',
		spec: {
			label: 'Cache results',
			icon: Database,
			// The worker takes the module's cache_ttl over the referenced script's, so a
			// module-level TTL must show here even for a workspace-script step.
			configured: (m, ctx) =>
				m.cache_ttl !== undefined || (isWorkspaceScript(m) && (ctx.referenced?.cache_ttl ?? 0) > 0),
			summarize: (m, ctx) => {
				const ttl = effectiveCacheTtl(m, ctx)
				if (ttl === undefined) return def('Off')
				return ttl > 0 ? cfg(formatDur(ttl)) : inv('No TTL set')
			}
		}
	},
	{
		key: 'debounce',
		spec: {
			label: 'Debounce',
			icon: Combine,
			configured: (m) => m.debouncing?.debounce_delay_s !== undefined,
			summarize: (m) => {
				const d = m.debouncing?.debounce_delay_s
				if (d === undefined) return def('Off')
				return d > 0 ? cfg(`${formatDur(d)} debounce`) : inv('No delay set')
			}
		}
	},
	{
		key: 'lifetime',
		spec: {
			label: 'Lifetime',
			icon: Trash2,
			configured: (m) => m.delete_after_secs != null,
			summarize: (m) => {
				const s = m.delete_after_secs
				if (s == null) return def('Off')
				return s === 0 ? cfg('Delete now') : cfg(`Delete after ${formatDur(s)}`)
			}
		}
	}
]

/** The settings that apply to this step, in canonical order.
 *  `referenced` supplies the workspace script's own settings for `script` steps,
 *  whose concurrency and cache live on the script rather than on the step. */
export function describeStepSettings(
	mod: FlowModule,
	referenced?: ScriptAdvancedSettingsFields
): StepSettingView[] {
	const ctx: Ctx = { referenced }
	return SPECS.filter(({ spec }) => spec.applies?.(mod) ?? true).map(({ key, spec }) => ({
		key,
		label: spec.label,
		tooltip: spec.tooltip?.(mod) ?? spec.label,
		icon: spec.icon,
		configured: spec.configured(mod, ctx),
		summary: spec.summarize(mod, ctx)
	}))
}

/** How a trigger step decides it has nothing to process. Stored on the step at
 *  creation, so changing it only affects newly created steps. */
// Falsy-or-empty, not `result == undefined`: a trigger returning nothing new may say so
// with any empty value, and stopping is always the right response.
const TRIGGER_STOP_EXPR = '!result || (Array.isArray(result) && result.length == 0)'

/** The config a setting is seeded with when it is switched on. Read by the setting
 *  editors and by every path that creates a step, so both agree. Settings absent from
 *  this map have no seeded config (the editor writes the value directly). */
const DEFAULTS = {
	skip: () => ({ expr: 'false' }),
	'early-stop': (kind?: 'trigger' | 'end') =>
		kind === 'trigger'
			? { expr: TRIGGER_STOP_EXPR, skip_if_stopped: true }
			: kind === 'end'
				? { expr: 'true', skip_if_stopped: false }
				: {
						expr: 'result == undefined',
						skip_if_stopped: false,
						error_message: undefined,
						error_include_result: false
					},
	suspend: () => ({ required_events: 1, timeout: 1800 }),
	sleep: () => ({ type: 'static' as const, value: 0 }),
	cache: () => 600,
	lifetime: () => 0,
	priority: () => 100
} satisfies Partial<Record<StepSettingKey, (kind?: 'trigger' | 'end') => unknown>>

export type SeededSettingKey = keyof typeof DEFAULTS

/** Seeded config for a setting. Typed per key, so an unhandled key is a compile
 *  error rather than a silent `undefined`. */
export function stepSettingDefaults<K extends SeededSettingKey>(
	key: K,
	kind?: 'trigger' | 'end'
): ReturnType<(typeof DEFAULTS)[K]> {
	return DEFAULTS[key](kind) as ReturnType<(typeof DEFAULTS)[K]>
}
