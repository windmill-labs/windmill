import type { FlowModule, FlowNote, FlowValue, OpenFlow, RawScript } from '$lib/gen'
import { forEachFlowModule } from '$lib/components/flows/dfs'
import { findModuleInFlow } from '$lib/components/flows/flowTree'
import {
	DEFAULT_NOTE_COLOR,
	MIN_NOTE_HEIGHT,
	MIN_NOTE_WIDTH,
	NoteColor
} from '$lib/components/graph/noteColors'
import type { InlineScriptSession } from './inlineScriptsUtils'

/** Allowed note/group color names — matches the NoteColor palette the note and
 * group editors use. The note renderer keys `NOTE_COLORS` by these exact names,
 * so other strings (hex codes, CSS colors) render with no styling at best and
 * break the color picker UI at worst. */
const ALLOWED_NOTE_COLORS = new Set<string>(Object.values(NoteColor))

/**
 * Estimate a free note's size from its markdown text so AI-created notes (which
 * omit `size`) don't clip. Free notes render at a fixed height and never grow to
 * fit their content — the renderer's text div overflows the node box — so the
 * geometry seeded here must already be tall enough to hold the text.
 *
 * The note renders at `text-xs` (12px / line-height 1.4 ≈ 17px per line) inside
 * `p-4` (16px padding on each side). We wrap each source line to the content
 * width, sum the wrapped line count, and clamp the result to sane bounds.
 */
function estimateFreeNoteSize(text: string): { width: number; height: number } {
	const width = MIN_NOTE_WIDTH
	const HORIZONTAL_PADDING = 32 // p-4 left + right
	const VERTICAL_PADDING = 32 // p-4 top + bottom
	const LINE_HEIGHT = 17 // 12px * 1.4
	const AVG_CHAR_WIDTH = 6.2 // approx width of a char at 12px
	const MAX_HEIGHT = 600

	const charsPerLine = Math.max(1, Math.floor((width - HORIZONTAL_PADDING) / AVG_CHAR_WIDTH))
	const sourceLines = (text ?? '').split('\n')
	const wrappedLineCount = sourceLines.reduce(
		(sum, line) => sum + Math.max(1, Math.ceil(line.length / charsPerLine)),
		0
	)

	const height = Math.min(
		MAX_HEIGHT,
		Math.max(MIN_NOTE_HEIGHT, Math.ceil(wrappedLineCount * LINE_HEIGHT) + VERTICAL_PADDING)
	)

	return { width, height }
}

type FlowLike = Pick<OpenFlow, 'value'> & {
	schema?: Record<string, any>
}

export type FlowGroup = NonNullable<FlowValue['groups']>[number]
export type { FlowNote }

export interface FlowJsonUpdate {
	modules?: FlowModule[]
	schema?: Record<string, any> | null
	preprocessorModule?: FlowModule | null
	failureModule?: FlowModule | null
	groups?: FlowGroup[] | null
	notes?: FlowNote[] | null
}

export interface FlowJsonUpdateResult {
	emptyInlineScriptModuleIds: string[]
}

export function updateRawScriptModuleContent(
	flow: FlowLike,
	id: string,
	code: string
): (FlowModule & { value: RawScript }) | undefined {
	const module = findModuleInFlow(flow.value, id)
	if (!module || module.value.type !== 'rawscript') {
		return undefined
	}

	const rawScriptModule = module as FlowModule & { value: RawScript }
	rawScriptModule.value.content = code
	return rawScriptModule
}

export function validateFlowGroups(
	rawGroups: unknown,
	moduleIds?: Set<string>
): FlowGroup[] | null {
	if (rawGroups == null) {
		return null
	}

	if (!Array.isArray(rawGroups)) {
		throw new Error('Flow groups must be an array')
	}

	return rawGroups.map((group, index) => {
		if (!group || typeof group !== 'object' || Array.isArray(group)) {
			throw new Error(`Invalid group at index ${index}: must be an object`)
		}
		const g = group as Record<string, unknown>
		if (typeof g.start_id !== 'string' || !g.start_id) {
			throw new Error(`Invalid group at index ${index}: start_id must be a non-empty string`)
		}
		if (typeof g.end_id !== 'string' || !g.end_id) {
			throw new Error(`Invalid group at index ${index}: end_id must be a non-empty string`)
		}
		if (moduleIds) {
			if (!moduleIds.has(g.start_id)) {
				throw new Error(
					`Invalid group at index ${index}: start_id "${g.start_id}" does not match any flow module`
				)
			}
			if (!moduleIds.has(g.end_id)) {
				throw new Error(
					`Invalid group at index ${index}: end_id "${g.end_id}" does not match any flow module`
				)
			}
		}
		if (g.color !== undefined && g.color !== null) {
			if (typeof g.color !== 'string' || !ALLOWED_NOTE_COLORS.has(g.color)) {
				throw new Error(
					`Invalid group at index ${index}: color must be one of ${[...ALLOWED_NOTE_COLORS].join(', ')}`
				)
			}
		}
		return g as unknown as FlowGroup
	})
}

/**
 * Validate the optional array of sticky notes the agent attached to the flow.
 * Notes are editor-only annotations and do not affect execution.
 *
 * `free` notes are the supported kind (standalone canvas annotations). The
 * `group` note type is deprecated for creation — the chat prompt steers the
 * agent toward `groups` instead — but it is still ACCEPTED here so flows that
 * already contain group notes round-trip cleanly through `patch_flow_json` /
 * `set_flow_json` rather than being rejected. When `moduleIds` is provided,
 * every `contained_node_ids` entry of a `group` note must reference an existing
 * module.
 *
 * A provided palette `color` is always preserved as-is; the default is only
 * filled in when a note omits `color` entirely (FlowNote.color is required).
 *
 * Free notes are also given a concrete `position` and `size` when missing. A
 * free note without geometry is not draggable/resizable in the editor (you'd
 * have to resize it first to give it a size) — UI-created notes always set both,
 * so agent-created notes must too. Provided geometry is preserved untouched.
 */
export function validateFlowNotes(rawNotes: unknown, moduleIds?: Set<string>): FlowNote[] | null {
	if (rawNotes == null) {
		return null
	}

	if (!Array.isArray(rawNotes)) {
		throw new Error('Flow notes must be an array')
	}

	const seenIds = new Set<string>()
	// Running y-cursor for auto-placed free notes so consecutive ones stack
	// below each other by their actual heights instead of overlapping.
	let autoStackY = 0
	const AUTO_STACK_GAP = 24
	return rawNotes.map((note, index) => {
		if (!note || typeof note !== 'object' || Array.isArray(note)) {
			throw new Error(`Invalid note at index ${index}: must be an object`)
		}
		const n = note as Record<string, unknown>
		if (typeof n.id !== 'string' || !n.id) {
			throw new Error(`Invalid note at index ${index}: id must be a non-empty string`)
		}
		if (seenIds.has(n.id)) {
			throw new Error(`Invalid note at index ${index}: duplicate note id "${n.id}"`)
		}
		seenIds.add(n.id)
		if (typeof n.text !== 'string') {
			throw new Error(`Invalid note at index ${index}: text must be a string`)
		}
		const type = n.type ?? 'free'
		if (type !== 'free' && type !== 'group') {
			throw new Error(`Invalid note at index ${index}: type must be "free" or "group"`)
		}
		if (n.color !== undefined && n.color !== null) {
			if (typeof n.color !== 'string' || !ALLOWED_NOTE_COLORS.has(n.color)) {
				throw new Error(
					`Invalid note at index ${index}: color must be one of ${[...ALLOWED_NOTE_COLORS].join(', ')}`
				)
			}
		}
		if (n.position !== undefined && n.position !== null) {
			const p = n.position as Record<string, unknown>
			if (
				typeof p !== 'object' ||
				Array.isArray(n.position) ||
				typeof p.x !== 'number' ||
				typeof p.y !== 'number'
			) {
				throw new Error(
					`Invalid note at index ${index}: position must be an object with numeric x and y`
				)
			}
		}
		if (n.size !== undefined && n.size !== null) {
			const s = n.size as Record<string, unknown>
			if (
				typeof s !== 'object' ||
				Array.isArray(n.size) ||
				typeof s.width !== 'number' ||
				typeof s.height !== 'number'
			) {
				throw new Error(
					`Invalid note at index ${index}: size must be an object with numeric width and height`
				)
			}
		}
		if (type === 'group' && n.contained_node_ids !== undefined) {
			if (
				!Array.isArray(n.contained_node_ids) ||
				n.contained_node_ids.some((id) => typeof id !== 'string')
			) {
				throw new Error(
					`Invalid note at index ${index}: contained_node_ids must be an array of strings`
				)
			}
			if (moduleIds) {
				for (const id of n.contained_node_ids as string[]) {
					if (!moduleIds.has(id)) {
						throw new Error(
							`Invalid note at index ${index}: contained_node_ids "${id}" does not match any flow module`
						)
					}
				}
			}
		}
		const normalized = {
			...(n as FlowNote),
			type,
			// Preserve a provided color; only seed the default when omitted.
			color: typeof n.color === 'string' ? n.color : DEFAULT_NOTE_COLOR
		} as FlowNote

		// Free notes need explicit geometry to be draggable/resizable. Size first
		// (from text, so tall notes get a tall box), then place any note missing a
		// position to the left of the flow column, stacking auto-placed notes by
		// their real heights so several generated notes don't overlap. Group notes
		// derive their layout from contained nodes, so they are left alone.
		if (type === 'free') {
			if (normalized.size == null) {
				normalized.size = estimateFreeNoteSize(normalized.text)
			}
			if (normalized.position == null) {
				normalized.position = {
					x: -(MIN_NOTE_WIDTH + 100),
					y: autoStackY
				}
				autoStackY += (normalized.size?.height ?? MIN_NOTE_HEIGHT) + AUTO_STACK_GAP
			}
		}

		return normalized
	})
}

export function applyFlowJsonUpdate(
	flow: FlowLike,
	inlineScriptSession: InlineScriptSession,
	{ modules, schema, preprocessorModule, failureModule, groups, notes }: FlowJsonUpdate
): FlowJsonUpdateResult {
	const emptyInlineScriptModuleIds = new Set<string>()

	if (modules !== undefined) {
		flow.value.modules = restoreFlowModules(
			modules,
			inlineScriptSession,
			emptyInlineScriptModuleIds
		)
	}

	if (schema !== undefined) {
		flow.schema = schema ?? undefined
	}

	if (preprocessorModule !== undefined) {
		flow.value.preprocessor_module =
			preprocessorModule === null
				? undefined
				: restoreFlowModule(preprocessorModule, inlineScriptSession, emptyInlineScriptModuleIds)
	}

	if (failureModule !== undefined) {
		flow.value.failure_module =
			failureModule === null
				? undefined
				: restoreFlowModule(failureModule, inlineScriptSession, emptyInlineScriptModuleIds)
	}

	if (groups !== undefined) {
		flow.value.groups = groups == null || groups.length === 0 ? undefined : groups
	}

	if (notes !== undefined) {
		flow.value.notes = notes == null || notes.length === 0 ? undefined : notes
	}

	return {
		emptyInlineScriptModuleIds: Array.from(emptyInlineScriptModuleIds)
	}
}

function restoreFlowModules(
	modules: FlowModule[],
	inlineScriptSession: InlineScriptSession,
	emptyInlineScriptModuleIds: Set<string>
): FlowModule[] {
	const restoredModules = inlineScriptSession.restoreInlineScriptReferences(modules)
	replaceNewInlineScriptRefsWithEmptyCode(restoredModules, emptyInlineScriptModuleIds)
	assertResolvedInlineScripts(restoredModules, inlineScriptSession)
	return restoredModules
}

function restoreFlowModule(
	module: FlowModule,
	inlineScriptSession: InlineScriptSession,
	emptyInlineScriptModuleIds: Set<string>
): FlowModule {
	const [restoredModule] = inlineScriptSession.restoreInlineScriptReferences([module])
	replaceNewInlineScriptRefsWithEmptyCode([restoredModule], emptyInlineScriptModuleIds)
	assertResolvedInlineScripts([restoredModule], inlineScriptSession)
	return restoredModule
}

function assertResolvedInlineScripts(
	modules: FlowModule[],
	inlineScriptSession: InlineScriptSession
): void {
	const unresolvedRefs = inlineScriptSession.findUnresolvedInlineScriptRefs(modules)
	if (unresolvedRefs.length > 0) {
		throw new Error(`Unresolved inline script references: ${unresolvedRefs.join(', ')}`)
	}
}

function replaceNewInlineScriptRefsWithEmptyCode(
	modules: FlowModule[],
	emptyInlineScriptModuleIds: Set<string>
): void {
	function replaceInlineScriptRefWithEmptyCode(ownerId: string, content: string): string {
		const match = content.match(/^inline_script\.(.+)$/)
		if (!match || match[1] !== ownerId) {
			return content
		}

		emptyInlineScriptModuleIds.add(ownerId)
		return ''
	}

	forEachFlowModule(modules, (module) => {
		if (module.value.type === 'rawscript' && module.value.content) {
			module.value.content = replaceInlineScriptRefWithEmptyCode(module.id, module.value.content)
		}
	})
}
