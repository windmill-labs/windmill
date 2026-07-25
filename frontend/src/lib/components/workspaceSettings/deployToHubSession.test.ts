import { describe, it, expect } from 'vitest'
import { canShareAsIframe, mergeShareState, type DeployItem } from './deployToHubSession.svelte'

function item(over: Partial<DeployItem> & Pick<DeployItem, 'key' | 'path' | 'kind'>): DeployItem {
	return { rec: 'none', ...over }
}

describe('canShareAsIframe', () => {
	it('allows low-code apps and app-table raw apps', () => {
		expect(canShareAsIframe(item({ key: 'app:f/a', path: 'f/a', kind: 'app' }))).toBe(true)
		expect(
			canShareAsIframe(item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app', appTable: true }))
		).toBe(true)
	})
	it('hides the action for legacy raw apps (raw_app table only)', () => {
		// Legacy entries from RawAppService carry no appTable flag; AppService can't load them.
		expect(canShareAsIframe(item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app' }))).toBe(false)
	})
	it('never offers the action for flows or scripts', () => {
		expect(canShareAsIframe(item({ key: 'flow:f/f', path: 'f/f', kind: 'flow' }))).toBe(false)
	})
})

describe('mergeShareState', () => {
	it('carries live public-share state from workspace items onto matching drafts', () => {
		const drafts = [item({ key: 'app:f/a', path: 'f/a', kind: 'app' })]
		const workspace = [
			item({
				key: 'app:f/a',
				path: 'f/a',
				kind: 'app',
				published: true,
				publicUrl: 'https://x/app'
			})
		]
		const merged = mergeShareState(drafts, workspace)
		expect(merged[0].published).toBe(true)
		expect(merged[0].publicUrl).toBe('https://x/app')
	})
	it('restores the app-table origin so app-table raw apps stay shareable', () => {
		const drafts = [item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app' })]
		const workspace = [item({ key: 'raw_app:f/r', path: 'f/r', kind: 'raw_app', appTable: true })]
		expect(canShareAsIframe(mergeShareState(drafts, workspace)[0])).toBe(true)
	})
	it('returns the same reference when nothing changes', () => {
		const drafts = [item({ key: 'flow:f/f', path: 'f/f', kind: 'flow' })]
		expect(mergeShareState(drafts, drafts)).toBe(drafts)
	})
	it('leaves drafts without a workspace match untouched', () => {
		const drafts = [item({ key: 'app:f/gone', path: 'f/gone', kind: 'app' })]
		const merged = mergeShareState(drafts, [item({ key: 'app:f/a', path: 'f/a', kind: 'app' })])
		expect(merged).toBe(drafts)
	})
})
