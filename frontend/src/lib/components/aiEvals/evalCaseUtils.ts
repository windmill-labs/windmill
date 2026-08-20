import type { EvalCase, NewEvalCase } from '$lib/gen'

/** The case being edited in the drawer, before it is either run or saved to a dataset. */
export type CaseDraft = NewEvalCase & { id?: string }

export function emptyCase(): CaseDraft {
	return { input: { user_message: '' } }
}

export function fromStoredCase(c: EvalCase): CaseDraft {
	const { created_at: _created_at, created_by: _created_by, ...rest } = c
	return rest
}

/** What the case list shows for a case that was never given a name. */
export function caseLabel(c: Pick<CaseDraft, 'name' | 'input'>): string {
	if (c.name) return c.name
	const message = c.input?.user_message?.trim()
	if (message) return message.length > 60 ? message.slice(0, 60) + '…' : message
	return 'Untitled case'
}
