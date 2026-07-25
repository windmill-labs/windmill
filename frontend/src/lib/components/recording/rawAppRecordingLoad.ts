/**
 * Validation for an app recording arriving from outside — an uploaded file or a
 * `?src=` URL. Both replay pages go through this: a recording is caller-supplied
 * data that the player indexes into and renders per step, so its shape, its
 * cardinality and its sizes all have to hold before anything mounts.
 */
import type { RawAppRecording } from './types'
import {
	MAX_RECORDED_STEPS,
	MAX_STEP_TEXT_CHARS,
	MAX_TOTAL_FRAME_CHARS,
	RAW_APP_INTERACTION_KINDS,
	type RawAppInteractionKind
} from './rawAppSnapshot'

/** Upper bound on a fetched recording (JSON of capped frames) so an arbitrary
 * origin can't exhaust the tab before validation runs. */
export const MAX_RECORDING_BYTES = 100 * 1024 * 1024

const isObject = (v: unknown): v is Record<string, unknown> =>
	typeof v === 'object' && v !== null && !Array.isArray(v)

const isShortText = (v: unknown, required = false) =>
	required
		? typeof v === 'string' && v.length <= MAX_STEP_TEXT_CHARS
		: v === undefined || (typeof v === 'string' && v.length <= MAX_STEP_TEXT_CHARS)

const isSize = (v: unknown) => typeof v === 'number' && Number.isFinite(v) && v > 0 && v <= 20000

/** True when `data` is a well-formed app recording this build can replay. */
export function isAppRecording(data: unknown): data is RawAppRecording {
	if (!isObject(data) || data.version !== 1 || data.type !== 'app') return false
	// Every frame is parsed and re-serialized before the iframe parses it again,
	// so a single huge frame would freeze the tab even under the download cap.
	const validFrames =
		Array.isArray(data.frames) &&
		data.frames.length <= 2 * MAX_RECORDED_STEPS + 1 &&
		data.frames.every((f) => typeof f === 'string') &&
		(data.frames as string[]).reduce((sum, f) => sum + f.length, 0) <= MAX_TOTAL_FRAME_CHARS
	if (!validFrames) return false
	const frameCount = (data.frames as string[]).length
	// An index must address a frame that exists; `undefined` stays legitimate for
	// a capture the recorder had to skip.
	const isIndex = (v: unknown) =>
		v === undefined || (typeof v === 'number' && Number.isInteger(v) && v >= 0 && v < frameCount)
	const validSteps =
		Array.isArray(data.steps) &&
		data.steps.length <= MAX_RECORDED_STEPS &&
		data.steps.every(
			(s: unknown) =>
				isObject(s) &&
				typeof s.t === 'number' &&
				RAW_APP_INTERACTION_KINDS.includes(s.kind as RawAppInteractionKind) &&
				isShortText(s.label, true) &&
				isShortText(s.target) &&
				isShortText(s.selector) &&
				isShortText(s.value) &&
				isIndex(s.before) &&
				isIndex(s.after)
		)
	// The viewport lands in the snapshot iframe's `style`, and the duration is
	// divided into every step timestamp by the timeline.
	const validViewport =
		isObject(data.viewport) && isSize(data.viewport.width) && isSize(data.viewport.height)
	const validDuration =
		typeof data.total_duration_ms === 'number' &&
		Number.isFinite(data.total_duration_ms) &&
		data.total_duration_ms >= 0
	const validHeader =
		isShortText(data.app_path) && isShortText(data.workspace) && isShortText(data.recorded_at, true)
	return validSteps && validViewport && validDuration && validHeader
}

/** Fetch a recording from `url`, enforcing the download cap while streaming. */
export async function fetchRecording(
	url: string,
	onProgress?: (loaded: number, total: number) => void
): Promise<unknown> {
	const res = await fetch(url)
	if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`)
	const total = Number(res.headers.get('content-length')) || 0
	if (total > MAX_RECORDING_BYTES) throw new Error(`Recording is too large (${total} bytes)`)
	const reader = res.body?.getReader()
	if (!reader) {
		const text = await res.text()
		if (text.length > MAX_RECORDING_BYTES) throw new Error('Recording exceeded the size limit')
		return JSON.parse(text)
	}
	const chunks: Uint8Array[] = []
	let loaded = 0
	for (;;) {
		const { done, value } = await reader.read()
		if (done) break
		if (!value) continue
		chunks.push(value)
		loaded += value.length
		if (loaded > MAX_RECORDING_BYTES) {
			await reader.cancel()
			throw new Error('Recording exceeded the size limit')
		}
		onProgress?.(loaded, total)
	}
	return JSON.parse(await new Blob(chunks as BlobPart[]).text())
}
