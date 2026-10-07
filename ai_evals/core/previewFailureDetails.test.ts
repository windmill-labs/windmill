import { describe, expect, it } from 'bun:test'
import { formatPreviewFailureDetails } from './previewFailureDetails'

describe('formatPreviewFailureDetails', () => {
	it('includes runtime logs and result excerpts for failed preview jobs', () => {
		const details = formatPreviewFailureDetails({
			workspaceId: 'ai-evals-crud-a1',
			jobId: 'job-123',
			logs: '\nTypeError: Cannot read properties of undefined\n    at listRecipes\n',
			result: { error: 'frontend called missing backend runnable' }
		})

		expect(details).toContain('workspace=ai-evals-crud-a1')
		expect(details).toContain('job=job-123')
		expect(details).toContain('TypeError: Cannot read properties of undefined')
		expect(details).toContain('frontend called missing backend runnable')
	})

	it('bounds noisy logs so check summaries stay readable', () => {
		const details = formatPreviewFailureDetails({
			workspaceId: 'workspace',
			jobId: 'job',
			logs: 'x'.repeat(100),
			maxCharacters: 20
		})

		expect(details).toContain(`logs=${'x'.repeat(19)}…`)
		expect(details.length).toBeLessThan(80)
	})
})
