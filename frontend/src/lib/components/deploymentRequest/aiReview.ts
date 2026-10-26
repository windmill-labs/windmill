import type { AIProviderModel, WorkspaceItemDiff } from '$lib/gen'
import { getNonStreamingCompletion } from '$lib/components/copilot/lib'
import { getItemValue } from '$lib/utils_workspace_deploy'
import type { Kind } from '$lib/utils_deployable'
import {
	formatFolderInstructions,
	instructionsCovering,
	listFolderInstructions,
	readFolderInstructionBody,
	type FolderInstruction
} from '$lib/components/copilot/chat/folderInstructions'

/**
 * Review instructions are resources of this type in the parent workspace: a markdown file
 * resource whose `value.content` applies to every changed item under the directory it lives in.
 */
export const REVIEW_INSTRUCTIONS_RESOURCE_TYPE = 'ai_review_instruction'

export type AiReviewSeverity = 'high' | 'medium' | 'low'

export type AiReviewFinding = {
	id: string
	kind: string
	path: string
	severity: AiReviewSeverity
	comment: string
}

export type AiReviewResult = {
	summary: string
	findings: AiReviewFinding[]
	/** Changed items whose contents did not fit in the prompt and were reviewed by name only. */
	omitted: string[]
	/** Changed items whose contents failed to load, so the model did not review them. */
	unreadable: string[]
	/** Review instructions of the parent that covered a changed item and were given to the model. */
	instructions: string[]
	/** Review instructions listed in the parent whose body failed to load. */
	skippedInstructions: string[]
	/** The parent's review instructions could not be listed, so none were applied. */
	instructionsUnavailable: boolean
}

// A variable's value is read decrypted and a resource's value holds credentials, so neither may
// reach the model: they are reviewed by name and change type only.
const VALUE_WITHHELD_KINDS = new Set(['variable', 'resource'])

const PER_SIDE_CHAR_CAP = 20_000
const TOTAL_CHAR_BUDGET = 160_000
const FETCH_BATCH = 6
// Under the Anthropic SDK's non-streaming pre-flight max_tokens limit.
const MAX_OUTPUT_TOKENS = 8_000

const SYSTEM_PROMPT = `You review a Windmill deployment request: a set of changes made in a fork workspace that will be merged into its parent workspace. Each changed item is given with its parent version (what is deployed now) and its fork version (what will be deployed). An item can be a script, flow, app, schedule, trigger, folder, resource type, datatable migration, variable or resource.

Report only issues a reviewer should act on before approving the merge:
- bugs and logic errors introduced by the change
- breaking changes: removed or renamed inputs, changed input types, changed outputs that callers may rely on
- risky operational changes: schedules retimed, triggers pointed at a different script or flow, folder owners or permissions, app policies, destructive migrations
- hardcoded secrets or credentials
- deletions that look unintended

Do not comment on style, naming, formatting or lockfiles, and do not praise. Do not repeat a finding for every item it touches. An item with no issue gets no finding.

Answer with JSON only, no prose around it, in this shape:
{"summary": "<one or two sentences on what the change does and its overall risk>", "findings": [{"kind": "<item kind as given>", "path": "<item path as given>", "severity": "high" | "medium" | "low", "comment": "<the issue and what to do about it, under 80 words>"}]}`

// Read from the parent, never the fork: the fork is the author's side, and its copy of an
// instruction is itself one of the changes under review. A failed listing reviews without
// instructions rather than failing the review, and is reported as `unavailable`.
async function reviewInstructions(
	diffs: WorkspaceItemDiff[],
	parentWorkspaceId: string
): Promise<{ text: string; applied: string[]; skipped: string[]; unavailable: boolean }> {
	let listed: FolderInstruction[]
	try {
		listed = await listFolderInstructions(parentWorkspaceId, REVIEW_INSTRUCTIONS_RESOURCE_TYPE)
	} catch (e) {
		console.error('Failed to list review instructions', e)
		return { text: '', applied: [], skipped: [], unavailable: true }
	}
	const covering = instructionsCovering(
		listed,
		diffs.map((d) => d.path)
	)
	const reads = await Promise.all(
		covering.map(async (instruction) => {
			try {
				return {
					instruction,
					body: await readFolderInstructionBody(parentWorkspaceId, instruction.path)
				}
			} catch (e) {
				console.error(`Failed to read review instructions ${instruction.path}`, e)
				return { instruction, body: undefined }
			}
		})
	)
	const blocks = reads.filter(
		(r): r is { instruction: FolderInstruction; body: string } => r.body !== undefined
	)
	// Hardcoded secrets and unintended deletions stay a floor: a folder's writers steer the
	// review of their own changes, so they must not be able to switch those checks off.
	const text =
		blocks.length === 0
			? ''
			: `\n\nThe parent workspace gives the review instructions below, each for the changed items under the folder in its scope. For those items they take precedence over the instructions above, and a nested folder's over its parent folder's. They may ask you to check more, check less or weigh severity differently, except that hardcoded secrets or credentials and deletions that look unintended are always reported. The answer is still the JSON above.\n\n${formatFolderInstructions(blocks)}`
	return {
		text,
		applied: blocks.map((b) => b.instruction.path),
		skipped: reads.filter((r) => r.body === undefined).map((r) => r.instruction.path),
		unavailable: false
	}
}

function changeType(diff: WorkspaceItemDiff): 'added' | 'deleted' | 'modified' {
	if (!diff.exists_in_source) return 'added'
	if (!diff.exists_in_fork) return 'deleted'
	return 'modified'
}

function render(value: unknown): string {
	if (value == null) return '(none)'
	// Flows and apps nest their inline scripts' lockfiles, so they are dropped at any depth.
	const text =
		typeof value === 'string'
			? value
			: JSON.stringify(value, (key, v) => (key === 'lock' ? undefined : v), 2)
	return text.length > PER_SIDE_CHAR_CAP
		? `${text.slice(0, PER_SIDE_CHAR_CAP)}\n… (truncated, ${text.length - PER_SIDE_CHAR_CAP} more characters)`
		: text
}

async function itemSection(
	diff: WorkspaceItemDiff,
	forkWorkspaceId: string,
	parentWorkspaceId: string
): Promise<{ text: string; unreadable: boolean }> {
	const change = changeType(diff)
	const header = `### ${diff.kind} ${diff.path} (${change})`
	if (VALUE_WITHHELD_KINDS.has(diff.kind)) {
		return { text: `${header}\nValue withheld from review.`, unreadable: false }
	}
	const kind = diff.kind as Kind
	const [parent, fork] = await Promise.all([
		change === 'added' ? undefined : getItemValue(kind, diff.path, parentWorkspaceId),
		change === 'deleted' ? undefined : getItemValue(kind, diff.path, forkWorkspaceId)
	])
	// `getItemValue` answers `{}` when a read fails, which would pass as an emptied item.
	if (isEmptyRead(parent) || isEmptyRead(fork)) {
		return { text: `${header}\nContents could not be read.`, unreadable: true }
	}
	return {
		text: `${header}\n#### Parent version\n${render(parent)}\n#### Fork version\n${render(fork)}`,
		unreadable: false
	}
}

function isEmptyRead(value: unknown): boolean {
	return value != null && typeof value === 'object' && Object.keys(value as object).length === 0
}

export function parseReviewAnswer(
	text: string,
	diffs: WorkspaceItemDiff[]
): Pick<AiReviewResult, 'summary' | 'findings'> {
	const start = text.indexOf('{')
	const end = text.lastIndexOf('}')
	if (start === -1 || end <= start) {
		throw new Error('The AI answer was not in the expected format')
	}
	const parsed = JSON.parse(text.slice(start, end + 1)) as {
		summary?: unknown
		findings?: unknown
	}
	// Anchoring a comment needs a (kind, path) the fork's diff actually has, so a finding naming
	// anything else is kept but unanchored rather than posted against a phantom item.
	const known = new Set(diffs.map((d) => `${d.kind}:${d.path}`))
	const findings: AiReviewFinding[] = []
	for (const [i, raw] of (Array.isArray(parsed.findings) ? parsed.findings : []).entries()) {
		if (!raw || typeof raw.comment !== 'string' || !raw.comment.trim()) continue
		const anchored = known.has(`${raw.kind}:${raw.path}`)
		findings.push({
			id: String(i),
			kind: anchored ? raw.kind : '',
			path: anchored ? raw.path : '',
			severity: ['high', 'medium', 'low'].includes(raw.severity) ? raw.severity : 'medium',
			comment: raw.comment.trim()
		})
	}
	const rank: Record<AiReviewSeverity, number> = { high: 0, medium: 1, low: 2 }
	findings.sort((a, b) => rank[a.severity] - rank[b.severity])
	return { summary: typeof parsed.summary === 'string' ? parsed.summary : '', findings }
}

/** Reviews the changes `diffs` carry from the fork into its parent with the workspace's AI model. */
export async function reviewDeployment(
	diffs: WorkspaceItemDiff[],
	forkWorkspaceId: string,
	parentWorkspaceId: string,
	abortController: AbortController,
	options?: {
		/** Defaults to the chat's current model. */
		model?: AIProviderModel
	}
): Promise<AiReviewResult> {
	const instructionsPromise = reviewInstructions(diffs, parentWorkspaceId)
	const included: string[] = []
	const omitted: string[] = []
	const unreadable: string[] = []
	let used = 0
	// Fetched a batch at a time so a large fork stops reading items once the budget is spent.
	// The first section that does not fit spends it: `used` alone can stay under the cap forever.
	let budgetSpent = false
	for (let i = 0; i < diffs.length; i += FETCH_BATCH) {
		const batch = diffs.slice(i, i + FETCH_BATCH)
		if (budgetSpent) {
			omitted.push(...batch.map((d) => `${d.kind}:${d.path}`))
			continue
		}
		const sections = await Promise.all(
			batch.map((d) => itemSection(d, forkWorkspaceId, parentWorkspaceId))
		)
		if (abortController.signal.aborted) throw new Error('aborted')
		sections.forEach(({ text: section, unreadable: failed }, j) => {
			if (failed) unreadable.push(`${batch[j].kind}:${batch[j].path}`)
			if (!budgetSpent && used + section.length <= TOTAL_CHAR_BUDGET) {
				included.push(section)
				used += section.length
			} else {
				budgetSpent = true
				omitted.push(`${batch[j].kind}:${batch[j].path}`)
			}
		})
	}
	let user = `Fork workspace: ${forkWorkspaceId}\nParent workspace: ${parentWorkspaceId}\n\n${included.join('\n\n')}`
	if (omitted.length > 0) {
		user += `\n\nAlso changed, contents not shown for size:\n${omitted.map((o) => `- ${o}`).join('\n')}`
	}

	const instructions = await instructionsPromise
	if (abortController.signal.aborted) throw new Error('aborted')
	const answer = await getNonStreamingCompletion(
		[
			{ role: 'system', content: SYSTEM_PROMPT + instructions.text },
			{ role: 'user', content: user }
		],
		abortController,
		// Bound to the fork: without `workspace` the call goes through whichever workspace the
		// app has switched to by the time the item reads finish.
		{
			maxTokensCap: MAX_OUTPUT_TOKENS,
			forceModelProvider: options?.model,
			workspace: forkWorkspaceId
		}
	)
	return {
		...parseReviewAnswer(answer ?? '', diffs),
		omitted,
		unreadable,
		instructions: instructions.applied,
		skippedInstructions: instructions.skipped,
		instructionsUnavailable: instructions.unavailable
	}
}
