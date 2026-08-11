import type { EvalCase, EvalCaseDraft, NewEvalCase } from '$lib/gen'

/** The case being edited in the drawer, before it is either run or saved to a dataset. */
export type CaseDraft = NewEvalCase & { id?: string }

export function emptyCase(): CaseDraft {
	return { input: { user_message: '' } }
}

export function fromStoredCase(c: EvalCase): CaseDraft {
	const { created_at: _created_at, created_by: _created_by, ...rest } = c
	return rest
}

/** Deep-copied, because the draft is edited in place and must not write back into the capture the
 *  caller still holds. Via JSON rather than `structuredClone`, which throws on the reactive proxy
 *  a capture arrives wrapped in; a capture is API JSON, so the round-trip is lossless. */
export function fromCaptureDraft(draft: EvalCaseDraft): CaseDraft {
	return JSON.parse(
		JSON.stringify({
			name: draft.name,
			input: draft.input,
			host_flow_path: draft.host_flow_path,
			tool_inputs: draft.tool_inputs,
			expected: draft.expected,
			source: draft.source
		})
	)
}

/** What the case list shows for a case that was never given a name. */
export function caseLabel(c: Pick<CaseDraft, 'name' | 'input'>): string {
	if (c.name) return c.name
	const message = c.input?.user_message?.trim()
	if (message) return message.length > 60 ? message.slice(0, 60) + '…' : message
	return 'Untitled case'
}

/**
 * The run that a case produces is a job whose path is `<agent>/<dataset>/<case>`, which is what
 * lets the run history be a plain jobs-list query instead of stored state. Agent paths long
 * enough to overflow `runnable_path` fall back to the agent alone server-side, so a missing
 * history here means "not filterable", not "never ran".
 */
export function caseRunPath(agentPath: string, datasetPath: string, caseId: string): string {
	return `${agentPath}/${datasetPath}/${caseId}`
}

/**
 * A draft reduced to what a case actually is, for comparing an edited draft against the case it
 * came from. The editor materializes keys the stored case omits (`messages: undefined` when there
 * is no conversation), so comparing the raw objects reports an untouched case as edited — which
 * would send it inline and strip the dataset/case stamp its run history depends on.
 */
export function comparableCase(draft: CaseDraft): unknown {
	return JSON.parse(
		JSON.stringify({
			name: draft.name || undefined,
			input: {
				user_message: draft.input?.user_message || undefined,
				user_attachments: draft.input?.user_attachments?.length
					? draft.input.user_attachments
					: undefined,
				messages: draft.input?.messages?.length ? draft.input.messages : undefined
			},
			host_flow_path: draft.host_flow_path || undefined,
			tool_inputs: draft.tool_inputs,
			expected: draft.expected,
			tags: draft.tags?.length ? draft.tags : undefined
		})
	)
}
