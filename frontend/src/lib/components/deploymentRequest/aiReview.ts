import type { AIProviderModel, WorkspaceItemDiff } from '$lib/gen'
import { getNonStreamingCompletion } from '$lib/components/copilot/lib'
import { getItemValue } from '$lib/utils_workspace_deploy'
import type { Kind } from '$lib/utils_deployable'
import {
	listFolderInstructions,
	readFolderInstructionBody
} from '$lib/components/copilot/chat/folderInstructions'
import {
	instructionsCovering,
	MAX_OUTPUT_TOKENS,
	parseReviewAnswer,
	renderItemValue,
	REVIEW_INSTRUCTIONS_RESOURCE_TYPE,
	reviewInstructionsSection,
	ReviewBudget,
	reviewSystemPrompt,
	VALUE_WITHHELD_KINDS,
	type AiReviewFinding,
	type FolderInstruction,
	type ReviewSubject
} from './aiReviewCore'

export {
	REVIEW_INSTRUCTIONS_RESOURCE_TYPE,
	type AiReviewFinding,
	type AiReviewSeverity
} from './aiReviewCore'

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

/** One changed item to review, its two sides read only once the review gets to it. */
export type ReviewItem = {
	kind: string
	path: string
	change: 'added' | 'deleted' | 'modified'
	/** Resolves `undefined`, or throws, when the contents could not be read. */
	load: () => Promise<{ before: unknown; after: unknown } | undefined>
}

/**
 * What a review covers: the changes a fork carries into its parent, or drafts not yet deployed
 * in their own workspace.
 */
export type ReviewScope =
	| {
			kind: 'deployment'
			diffs: WorkspaceItemDiff[]
			forkWorkspaceId: string
			parentWorkspaceId: string
	  }
	| { kind: 'drafts'; workspaceId: string; items: ReviewItem[] }

/** The workspace whose items change: fixes belong there, and the AI call is bound to it. */
export function scopeWorkspace(scope: ReviewScope): string {
	return scope.kind === 'deployment' ? scope.forkWorkspaceId : scope.workspaceId
}

/**
 * The workspace review instructions are read from. A fork's changes are reviewed under its
 * parent's, never the fork's own: the fork is the author's side, and its copy of an instruction
 * is itself one of the changes under review. Drafts are reviewed under their workspace's
 * deployed instructions, for the same reason a draft of one does not apply.
 */
export function instructionsWorkspace(scope: ReviewScope): string {
	return scope.kind === 'deployment' ? scope.parentWorkspaceId : scope.workspaceId
}

export function scopeItems(scope: ReviewScope): ReviewItem[] {
	if (scope.kind === 'drafts') return scope.items
	const { forkWorkspaceId, parentWorkspaceId } = scope
	return scope.diffs.map((diff) => {
		const change = changeType(diff)
		return {
			kind: diff.kind,
			path: diff.path,
			change,
			load: async () => {
				const kind = diff.kind as Kind
				const [before, after] = await Promise.all([
					change === 'added' ? undefined : getItemValue(kind, diff.path, parentWorkspaceId),
					change === 'deleted' ? undefined : getItemValue(kind, diff.path, forkWorkspaceId)
				])
				// `getItemValue` answers `{}` when a read fails, which would pass as an emptied item.
				return isEmptyRead(before) || isEmptyRead(after) ? undefined : { before, after }
			}
		}
	})
}

const FETCH_BATCH = 6

// A failed listing reviews without instructions rather than failing the review, and is reported
// as `unavailable`.
async function reviewInstructions(
	subject: ReviewSubject,
	paths: string[],
	workspaceId: string
): Promise<{ text: string; applied: string[]; skipped: string[]; unavailable: boolean }> {
	let listed: FolderInstruction[]
	try {
		listed = await listFolderInstructions(workspaceId, REVIEW_INSTRUCTIONS_RESOURCE_TYPE)
	} catch (e) {
		console.error('Failed to list review instructions', e)
		return { text: '', applied: [], skipped: [], unavailable: true }
	}
	const covering = instructionsCovering(listed, paths)
	const reads = await Promise.all(
		covering.map(async (instruction) => {
			try {
				return {
					instruction,
					body: await readFolderInstructionBody(workspaceId, instruction.path)
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
	return {
		text: reviewInstructionsSection(subject, blocks),
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

const SIDES: Record<ReviewScope['kind'], [string, string]> = {
	deployment: ['Parent version', 'Fork version'],
	drafts: ['Deployed version', 'Draft version']
}

async function itemSection(
	item: ReviewItem,
	sides: [string, string]
): Promise<{ text: string; unreadable: boolean }> {
	const header = `### ${item.kind} ${item.path} (${item.change})`
	if (VALUE_WITHHELD_KINDS.has(item.kind)) {
		return { text: `${header}\nValue withheld from review.`, unreadable: false }
	}
	const values = await item.load().catch((e) => {
		console.error(`Failed to read ${item.kind} ${item.path} for review`, e)
		return undefined
	})
	if (!values) {
		return { text: `${header}\nContents could not be read.`, unreadable: true }
	}
	const before = item.change === 'added' ? undefined : values.before
	const after = item.change === 'deleted' ? undefined : values.after
	return {
		text: `${header}\n#### ${sides[0]}\n${renderItemValue(before)}\n#### ${sides[1]}\n${renderItemValue(after)}`,
		unreadable: false
	}
}

function isEmptyRead(value: unknown): boolean {
	return value != null && typeof value === 'object' && Object.keys(value as object).length === 0
}

/** Reviews the changes `scope` covers with the workspace's AI model. */
export async function reviewChanges(
	scope: ReviewScope,
	abortController: AbortController,
	options?: {
		/** Defaults to the chat's current model. */
		model?: AIProviderModel
	}
): Promise<AiReviewResult> {
	const subject: ReviewSubject = scope.kind === 'deployment' ? 'deployment_request' : 'draft'
	const workspaceId = scopeWorkspace(scope)
	const items = scopeItems(scope)
	const instructionsPromise = reviewInstructions(
		subject,
		items.map((i) => i.path),
		instructionsWorkspace(scope)
	)
	const budget = new ReviewBudget()
	const unreadable: string[] = []
	// Fetched a batch at a time so a large review stops reading items once the budget is spent.
	for (let i = 0; i < items.length; i += FETCH_BATCH) {
		const batch = items.slice(i, i + FETCH_BATCH)
		if (budget.spent) {
			budget.omitted.push(...batch.map((d) => `${d.kind}:${d.path}`))
			continue
		}
		const sections = await Promise.all(batch.map((d) => itemSection(d, SIDES[scope.kind])))
		if (abortController.signal.aborted) throw new Error('aborted')
		sections.forEach(({ text, unreadable: failed }, j) => {
			const key = `${batch[j].kind}:${batch[j].path}`
			if (failed) unreadable.push(key)
			budget.add(key, text)
		})
	}

	const instructions = await instructionsPromise
	if (abortController.signal.aborted) throw new Error('aborted')
	const answer = await getNonStreamingCompletion(
		[
			{ role: 'system', content: reviewSystemPrompt(subject) + instructions.text },
			{
				role: 'user',
				content: budget.userPrompt(
					scope.kind === 'deployment'
						? `Fork workspace: ${scope.forkWorkspaceId}\nParent workspace: ${scope.parentWorkspaceId}`
						: `Workspace: ${scope.workspaceId}`
				)
			}
		],
		abortController,
		// Bound to the reviewed workspace: without `workspace` the call goes through whichever
		// workspace the app has switched to by the time the item reads finish.
		{
			maxTokensCap: MAX_OUTPUT_TOKENS,
			forceModelProvider: options?.model,
			workspace: workspaceId
		}
	)
	return {
		...parseReviewAnswer(answer ?? '', {
			items: new Set(items.map((d) => `${d.kind}:${d.path}`))
		}),
		omitted: budget.omitted,
		unreadable,
		instructions: instructions.applied,
		skippedInstructions: instructions.skipped,
		instructionsUnavailable: instructions.unavailable
	}
}
