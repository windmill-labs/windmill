import type { WorkspaceItemDiff } from '$lib/gen'
import { getNonStreamingCompletion } from '$lib/components/copilot/lib'
import { getItemValue } from '$lib/utils_workspace_deploy'
import type { Kind } from '$lib/utils_deployable'

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
}

// A variable's value is read decrypted and a resource's value holds credentials, so neither may
// reach the model: they are reviewed by name and change type only.
const VALUE_WITHHELD_KINDS = new Set(['variable', 'resource'])

const PER_SIDE_CHAR_CAP = 20_000
const TOTAL_CHAR_BUDGET = 160_000
// Under the Anthropic SDK's non-streaming pre-flight max_tokens limit.
const MAX_OUTPUT_TOKENS = 8_000

const SYSTEM_PROMPT = `You review a Windmill deployment request: a set of changes made in a fork workspace that will be merged into its parent workspace. Each changed item is given with its parent version (what is deployed now) and its fork version (what will be deployed). An item can be a script, flow, app, schedule, trigger, folder, resource type, datatable migration, variable or resource.

Report only issues a reviewer should act on before approving the merge:
- bugs and logic errors introduced by the change
- breaking changes: removed or renamed inputs, changed input types, changed outputs that callers may rely on
- risky operational changes: schedules or triggers enabled, disabled or retimed, permissions, on-behalf-of identity, folder ownership, destructive migrations
- hardcoded secrets or credentials
- deletions that look unintended

Do not comment on style, naming, formatting or lockfiles, and do not praise. Do not repeat a finding for every item it touches. An item with no issue gets no finding.

Answer with JSON only, no prose around it, in this shape:
{"summary": "<one or two sentences on what the change does and its overall risk>", "findings": [{"kind": "<item kind as given>", "path": "<item path as given>", "severity": "high" | "medium" | "low", "comment": "<the issue and what to do about it, under 80 words>"}]}`

function changeType(diff: WorkspaceItemDiff): 'added' | 'deleted' | 'modified' {
	if (!diff.exists_in_source) return 'added'
	if (!diff.exists_in_fork) return 'deleted'
	return 'modified'
}

function render(value: unknown): string {
	if (value == null) return '(none)'
	if (typeof value === 'object' && value && 'lock' in value) {
		const { lock: _, ...rest } = value as Record<string, unknown>
		value = rest
	}
	const text = typeof value === 'string' ? value : JSON.stringify(value, null, 2)
	return text.length > PER_SIDE_CHAR_CAP
		? `${text.slice(0, PER_SIDE_CHAR_CAP)}\n… (truncated, ${text.length - PER_SIDE_CHAR_CAP} more characters)`
		: text
}

async function itemSection(
	diff: WorkspaceItemDiff,
	forkWorkspaceId: string,
	parentWorkspaceId: string
): Promise<string> {
	const change = changeType(diff)
	const header = `### ${diff.kind} ${diff.path} (${change})`
	if (VALUE_WITHHELD_KINDS.has(diff.kind)) {
		return `${header}\nValue withheld from review.`
	}
	const kind = diff.kind as Kind
	const [parent, fork] = await Promise.all([
		change === 'added' ? undefined : getItemValue(kind, diff.path, parentWorkspaceId),
		change === 'deleted' ? undefined : getItemValue(kind, diff.path, forkWorkspaceId)
	])
	return `${header}\n#### Parent version\n${render(parent)}\n#### Fork version\n${render(fork)}`
}

export function parseReviewAnswer(
	text: string,
	diffs: WorkspaceItemDiff[]
): Omit<AiReviewResult, 'omitted'> {
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
	abortController: AbortController
): Promise<AiReviewResult> {
	const sections = await Promise.all(
		diffs.map((d) => itemSection(d, forkWorkspaceId, parentWorkspaceId))
	)
	const included: string[] = []
	const omitted: string[] = []
	let used = 0
	sections.forEach((section, i) => {
		if (used + section.length <= TOTAL_CHAR_BUDGET) {
			included.push(section)
			used += section.length
		} else {
			omitted.push(`${diffs[i].kind}:${diffs[i].path}`)
		}
	})
	let user = `Fork workspace: ${forkWorkspaceId}\nParent workspace: ${parentWorkspaceId}\n\n${included.join('\n\n')}`
	if (omitted.length > 0) {
		user += `\n\nAlso changed, contents not shown for size:\n${omitted.map((o) => `- ${o}`).join('\n')}`
	}

	const answer = await getNonStreamingCompletion(
		[
			{ role: 'system', content: SYSTEM_PROMPT },
			{ role: 'user', content: user }
		],
		abortController,
		{ maxTokensCap: MAX_OUTPUT_TOKENS }
	)
	return { ...parseReviewAnswer(answer ?? '', diffs), omitted }
}
