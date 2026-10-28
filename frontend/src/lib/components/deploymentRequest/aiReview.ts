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
	type FolderInstruction
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

const FETCH_BATCH = 6

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
	return {
		text: reviewInstructionsSection('deployment_request', blocks),
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
		text: `${header}\n#### Parent version\n${renderItemValue(parent)}\n#### Fork version\n${renderItemValue(fork)}`,
		unreadable: false
	}
}

function isEmptyRead(value: unknown): boolean {
	return value != null && typeof value === 'object' && Object.keys(value as object).length === 0
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
	const budget = new ReviewBudget()
	const unreadable: string[] = []
	// Fetched a batch at a time so a large fork stops reading items once the budget is spent.
	for (let i = 0; i < diffs.length; i += FETCH_BATCH) {
		const batch = diffs.slice(i, i + FETCH_BATCH)
		if (budget.spent) {
			budget.omitted.push(...batch.map((d) => `${d.kind}:${d.path}`))
			continue
		}
		const sections = await Promise.all(
			batch.map((d) => itemSection(d, forkWorkspaceId, parentWorkspaceId))
		)
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
			{ role: 'system', content: reviewSystemPrompt('deployment_request') + instructions.text },
			{
				role: 'user',
				content: budget.userPrompt(
					`Fork workspace: ${forkWorkspaceId}\nParent workspace: ${parentWorkspaceId}`
				)
			}
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
		...parseReviewAnswer(answer ?? '', {
			items: new Set(diffs.map((d) => `${d.kind}:${d.path}`))
		}),
		omitted: budget.omitted,
		unreadable,
		instructions: instructions.applied,
		skippedInstructions: instructions.skipped,
		instructionsUnavailable: instructions.unavailable
	}
}
