import { describe, it, expect } from 'vitest'
import { navPageFor } from './navPages'

describe('navPageFor', () => {
	it('names the page a route belongs to, under the app base', () => {
		expect(navPageFor('/base/runs', '/base')?.label).toBe('Runs')
	})

	it('keeps an item route on its page (longest prefix wins)', () => {
		expect(navPageFor('/runs/f/demo/x', '')?.label).toBe('Runs')
	})

	it('does not let Home swallow every route', () => {
		expect(navPageFor('/audit_logs', '')?.label).toBe('Audit logs')
		expect(navPageFor('/', '')?.label).toBe('Home')
	})

	it('falls back to the first segment for a route the table does not name', () => {
		expect(navPageFor('/service_logs/x', '')).toMatchObject({
			label: 'Service logs',
			path: '/service_logs'
		})
	})
})
