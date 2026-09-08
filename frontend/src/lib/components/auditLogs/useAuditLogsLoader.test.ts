import { describe, expect, it } from 'vitest'
import { computeFirstBatch } from './useAuditLogsLoader.svelte'

describe('computeFirstBatch', () => {
	it('starts at the page itself when nothing is batched', () => {
		expect(computeFirstBatch(3, 100, 100)).toEqual({ firstPage: 3, skipFirst: 0 })
	})

	it('lands exactly on the page start when the batch size divides the offset', () => {
		expect(computeFirstBatch(1, 100, 25)).toEqual({ firstPage: 1, skipFirst: 0 })
		expect(computeFirstBatch(3, 100, 25)).toEqual({ firstPage: 9, skipFirst: 0 })
		expect(computeFirstBatch(2, 100, 1)).toEqual({ firstPage: 101, skipFirst: 0 })
	})

	it('drops the rows before the page start when it does not', () => {
		// page 2 of 100 starts at offset 100, batches of 30 can only land on offset 90
		expect(computeFirstBatch(2, 100, 30)).toEqual({ firstPage: 4, skipFirst: 10 })
	})
})
