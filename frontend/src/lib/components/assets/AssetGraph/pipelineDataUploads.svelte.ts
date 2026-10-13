import { untrack } from 'svelte'
import type { AssetGraphResponse } from './types'

type Staged = { args: Record<string, any>; valid: boolean }

// An S3Object-shaped value (the file picker writes `{ s3: '<path>' }`).
function isS3Object(v: any): boolean {
	return !!v && typeof v === 'object' && !Array.isArray(v) && 's3' in v
}

// Whether a staged value carries actual data. Covers the two shapes a
// data-upload entry takes: an S3Object (file picker → `{ s3: '<path>' }`) and
// a plain required input (e.g. a JSON array pasted into the run form).
function hasMeaningfulValue(v: any): boolean {
	if (v == null) return false
	if (typeof v === 'string') return v.length > 0
	if (Array.isArray(v)) return v.length > 0
	if (typeof v === 'object')
		return isS3Object(v) ? typeof v.s3 === 'string' && v.s3.length > 0 : Object.keys(v).length > 0
	return true // numbers / booleans count as provided
}

/**
 * Staged run-form input for the graph's data-upload entry scripts, keyed by path.
 * Lifted out of the (transient, per-selection) run form so the uploaded/entered
 * data persists across selection changes: it drives each entry node's green
 * "ready" state and seeds a run with that input. `valid` is the run form's
 * full-schema validity (all required fields satisfied).
 */
export function usePipelineDataUploads(getGraph: () => AssetGraphResponse) {
	let staged = $state<Record<string, Staged>>({})

	// Scripts that are data-upload entry points (a `data_upload` trigger in the
	// displayed graph). They can't auto-run — they need their data first.
	const entryPaths = $derived(
		new Set(
			getGraph()
				.triggers.filter((t) => t.trigger_kind === 'data_upload' && t.runnable_kind === 'script')
				.map((t) => t.runnable_path)
		)
	)

	// Ready once the user actually provided its data, not just opened the form. Two
	// guards: the form's own full-schema `valid` (every required field — including
	// non-file ones — is satisfied), AND that any declared S3Object file field
	// actually carries a file (the picker can leave an empty `{ s3: '' }` on a
	// non-required file field, which `valid` alone wouldn't catch).
	function isReady(path: string): boolean {
		const s = staged[path]
		if (!s || !s.valid) return false
		const values = Object.values(s.args)
		const s3s = values.filter(isS3Object)
		if (s3s.length > 0) return s3s.every(hasMeaningfulValue)
		return values.some(hasMeaningfulValue)
	}

	const readyPaths = $derived(new Set([...entryPaths].filter((p) => isReady(p))))

	return {
		get entryPaths(): ReadonlySet<string> {
			return entryPaths
		},
		/** Entry scripts with their data provided — drives the green node treatment. */
		get readyPaths(): ReadonlySet<string> {
			return readyPaths
		},
		isReady,
		argsFor: (path: string): Record<string, any> | undefined => staged[path]?.args,
		// Persist the run form's args + validity, but only for data-upload entries —
		// other run forms (partitioned producers) run their own way and must not be
		// mistaken for a staged upload. Idempotent: the run form re-emits on every
		// keystroke/validation pass, so bail when the value is unchanged — otherwise
		// each emit would reassign `staged`, giving `readyPaths` a fresh Set identity
		// that re-syncs the canvas and re-fires the form's emit effect
		// (effect_update_depth_exceeded).
		// Called from the run forms' own effects, so its reads are untracked: a form
		// must not re-run because the store it just wrote to changed.
		stage: (path: string, args: Record<string, any>, valid: boolean) =>
			untrack(() => {
				if (!entryPaths.has(path)) return
				const prev = staged[path]
				if (prev && prev.valid === valid && JSON.stringify(prev.args) === JSON.stringify(args))
					return
				staged = { ...staged, [path]: { args, valid } }
			})
	}
}
