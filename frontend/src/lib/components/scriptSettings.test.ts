import { describe, it, expect } from 'vitest'
import { getActiveScriptSettingsBadges } from './scriptSettings'

describe('getActiveScriptSettingsBadges', () => {
	it('returns no badges for undefined or empty settings', () => {
		expect(getActiveScriptSettingsBadges(undefined)).toEqual([])
		expect(getActiveScriptSettingsBadges({})).toEqual([])
	})

	it('only surfaces settings that are actually active', () => {
		const keys = getActiveScriptSettingsBadges({
			concurrent_limit: 3,
			concurrency_time_window_s: 60,
			cache_ttl: 600,
			timeout: 120,
			priority: 50,
			tag: 'gpu'
		}).map((b) => b.key)
		expect(keys).toEqual(['concurrency', 'cache', 'timeout', 'priority', 'tag'])
	})

	it('treats a zero/absent priority and non-positive debounce as inactive', () => {
		const keys = getActiveScriptSettingsBadges({
			priority: 0,
			debounce_delay_s: 0
		}).map((b) => b.key)
		expect(keys).toEqual([])
	})

	it('treats non-positive concurrency limits, timeouts and cache ttl as inactive (legacy zero rows)', () => {
		const keys = getActiveScriptSettingsBadges({
			concurrent_limit: 0,
			timeout: 0,
			cache_ttl: 0
		}).map((b) => b.key)
		expect(keys).toEqual([])
	})

	it('keeps delete_after_secs of 0 active (immediate deletion is a real setting)', () => {
		const badge = getActiveScriptSettingsBadges({ delete_after_secs: 0 })
		expect(badge.map((b) => b.key)).toEqual(['delete_after_use'])
		expect(badge[0].detail).toContain('immediately')
	})

	it('pluralizes the concurrency detail correctly', () => {
		expect(getActiveScriptSettingsBadges({ concurrent_limit: 1 })[0].detail).toContain(
			'Max 1 execution'
		)
		expect(getActiveScriptSettingsBadges({ concurrent_limit: 2 })[0].detail).toContain(
			'Max 2 executions'
		)
	})
})
