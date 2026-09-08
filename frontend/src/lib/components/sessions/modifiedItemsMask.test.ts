import { describe, it, expect } from 'vitest'
import {
	maskKey,
	forkDiffKindToUserDraftKind,
	diffInMask,
	maskHasDraftRow
} from './modifiedItemsMask'
import type { WorkspaceItemDiff } from '$lib/gen'

// The fork-diff → user-draft kind bridge must stay the inverse of
// CompareDrafts.toLayoutKind (different taxonomies: `http_trigger` vs
// `trigger_http`, `schedule` vs `trigger_schedule`). A drift here silently breaks
// the chat-scoped mask join, so pin the table.

describe('maskKey', () => {
	it('joins kind and path with a colon', () => {
		expect(maskKey('script', 'u/admin/foo')).toBe('script:u/admin/foo')
		expect(maskKey('raw_app', 'f/bar/app')).toBe('raw_app:f/bar/app')
	})
})

describe('forkDiffKindToUserDraftKind', () => {
	it('maps trigger/schedule/app fork-diff kinds to their user-draft kind', () => {
		const cases: Array<[WorkspaceItemDiff['kind'], string]> = [
			['schedule', 'trigger_schedule'],
			['http_trigger', 'trigger_http'],
			['websocket_trigger', 'trigger_websocket'],
			['kafka_trigger', 'trigger_kafka'],
			['nats_trigger', 'trigger_nats'],
			['postgres_trigger', 'trigger_postgres'],
			['mqtt_trigger', 'trigger_mqtt'],
			['sqs_trigger', 'trigger_sqs'],
			['gcp_trigger', 'trigger_gcp'],
			['azure_trigger', 'trigger_azure'],
			['email_trigger', 'trigger_default_email'],
			['app', 'raw_app']
		]
		for (const [forkKind, expected] of cases) {
			expect(forkDiffKindToUserDraftKind(forkKind)).toBe(expected)
		}
	})

	it('passes identity kinds through unchanged', () => {
		for (const k of ['script', 'flow', 'raw_app', 'resource', 'variable'] as const) {
			expect(forkDiffKindToUserDraftKind(k)).toBe(k)
		}
	})

	it('returns undefined for kinds with no user-draft equivalent', () => {
		expect(forkDiffKindToUserDraftKind('resource_type')).toBeUndefined()
		expect(forkDiffKindToUserDraftKind('folder')).toBeUndefined()
	})
})

describe('diffInMask', () => {
	const diff = (kind: WorkspaceItemDiff['kind'], path: string): WorkspaceItemDiff =>
		({ kind, path }) as WorkspaceItemDiff

	it('matches a fork diff against the mask via the bridged kind', () => {
		const mask = new Set(['trigger_http:f/foo/route', 'script:u/me/s'])
		expect(diffInMask(diff('http_trigger', 'f/foo/route'), mask)).toBe(true)
		expect(diffInMask(diff('script', 'u/me/s'), mask)).toBe(true)
	})

	it('matches a legacy app diff under both its identity and bridged mask keys', () => {
		expect(diffInMask(diff('app', 'u/me/legacy'), new Set(['app:u/me/legacy']))).toBe(true)
		expect(diffInMask(diff('app', 'u/me/legacy'), new Set(['raw_app:u/me/legacy']))).toBe(true)
		expect(diffInMask(diff('app', 'u/me/legacy'), new Set(['app:u/me/other']))).toBe(false)
	})

	it('does not match when path or kind differ, or kind has no equivalent', () => {
		const mask = new Set(['trigger_http:f/foo/route'])
		expect(diffInMask(diff('http_trigger', 'f/other/route'), mask)).toBe(false)
		expect(diffInMask(diff('script', 'f/foo/route'), mask)).toBe(false)
		expect(diffInMask(diff('folder', 'f/foo/route'), mask)).toBe(false)
	})
})

describe('maskHasDraftRow', () => {
	it('matches by storage path or, for a parked live draft, by its visible draft_path', () => {
		const mask = new Set(['script:u/me/my_script'])
		expect(maskHasDraftRow(mask, { kind: 'script', path: 'u/me/my_script' })).toBe(true)
		expect(
			maskHasDraftRow(mask, {
				kind: 'script',
				path: 'u/me/draft_123',
				draft_path: 'u/me/my_script'
			})
		).toBe(true)
	})

	it('does not match a different path or kind', () => {
		const mask = new Set(['script:u/me/my_script'])
		expect(maskHasDraftRow(mask, { kind: 'script', path: 'u/me/other' })).toBe(false)
		expect(
			maskHasDraftRow(mask, { kind: 'flow', path: 'u/me/draft_123', draft_path: 'u/me/my_script' })
		).toBe(false)
	})
})
