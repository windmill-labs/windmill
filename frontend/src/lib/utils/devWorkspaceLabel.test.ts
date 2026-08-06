import { describe, it, expect } from 'vitest'
import { devLabelError } from './devWorkspaceLabel'

// `devLabelError` gates the submit button, so it must not be stricter than
// `normalize_dev_workspace_label` in the backend: a label the API would accept but this rejects is
// a label the user cannot enter at all. Same cases as that function's Rust tests.
describe('devLabelError', () => {
	it('accepts the offered labels and custom environment names', () => {
		for (const label of ['dev', 'staging', 'uat', 'pre-prod', 'qa_2', 'v1.2', ' uat ']) {
			expect(devLabelError(label), label).toBeUndefined()
		}
	})

	it('rejects what git will not take as a branch', () => {
		for (const label of [
			'',
			'feature/uat',
			'UAT',
			'-uat',
			'.uat',
			'u..at',
			'uat.',
			'uat.lock',
			'uat env',
			'uat~1',
			'u'.repeat(31),
			'wm-fork',
			'wm_deploy'
		]) {
			expect(devLabelError(label), label).toBeTruthy()
		}
	})
})
