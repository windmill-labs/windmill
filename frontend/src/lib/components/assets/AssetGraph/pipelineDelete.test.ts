import { afterEach, describe, expect, it, vi } from 'vitest'
import { DraftService, ScriptService } from '$lib/gen'
import { planPipelineDelete } from './pipelineDelete'

afterEach(() => vi.restoreAllMocks())

describe('planPipelineDelete', () => {
	it("takes only the pipeline's own triggers and addresses draft-only nodes by storage key", async () => {
		vi.spyOn(ScriptService, 'listRunnables').mockResolvedValue({
			items: [
				{ type: 'script', path: 'f/sales/ingest', auto_kind: 'pipeline' },
				{ type: 'script', path: 'f/sales/report' },
				{
					type: 'script',
					path: 'u/alice/draft_abc',
					draft_only: true,
					draft_path: 'f/sales/unpublished',
					auto_kind: 'pipeline'
				}
			]
		} as any)
		vi.spyOn(DraftService, 'getOwnDraft').mockResolvedValue(undefined as any)
		const trigger = (runnable_kind: string, runnable_path: string, path: string) => ({
			trigger_kind: 'schedule',
			runnable_kind,
			runnable_path,
			path
		})
		vi.stubGlobal(
			'fetch',
			vi.fn().mockResolvedValue({
				ok: true,
				json: async () => ({
					triggers: [
						trigger('script', 'f/sales/ingest', 'f/sales/ingest_schedule'),
						trigger('script', 'f/sales/report', 'f/sales/report_schedule'),
						trigger('flow', 'f/sales/ingest', 'f/sales/flow_schedule')
					]
				})
			})
		)

		const plan = await planPipelineDelete('w', 'sales')

		expect(plan.triggers).toEqual([{ kind: 'schedule', path: 'f/sales/ingest_schedule' }])
		expect(plan.scripts).toEqual([
			{ path: 'f/sales/ingest', displayPath: 'f/sales/ingest', draftOnly: false },
			{ path: 'u/alice/draft_abc', displayPath: 'f/sales/unpublished', draftOnly: true }
		])
		vi.unstubAllGlobals()
	})
})
