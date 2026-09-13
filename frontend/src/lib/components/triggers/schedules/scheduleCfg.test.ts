import { describe, it, expect } from 'vitest'
import {
	draftOnlyScheduleCfg,
	normalizeScheduleCfg,
	scheduleCfgOf,
	scheduleFormOf
} from './scheduleCfg'

// A deployed schedule as the API returns it: server fields the form never shows, a short cron.
const deployed = {
	path: 'u/me/s',
	schedule: '0 0 12 * *',
	timezone: 'UTC',
	script_path: 'u/me/script',
	is_flow: false,
	args: { x: 'a' },
	enabled: true,
	on_failure: 'script/hub/1/workspace-or-schedule-error-handler-slack',
	on_failure_extra_args: { channel: '#alerts' },
	on_recovery: null,
	summary: '',
	extra_perms: {},
	edited_by: 'me',
	edited_at: '2026-09-11T00:00:00Z',
	workspace_id: 'w',
	email: 'me@example.com'
}

describe('schedule config normalization', () => {
	// The deployed side is kept in this shape and compared with what the form hands back: if the
	// two drift, a schedule reads as edited the moment it is opened.
	it('is what the form hands back after loading it, and idempotent', () => {
		const once = normalizeScheduleCfg(deployed)
		expect(scheduleCfgOf(scheduleFormOf(once))).toEqual(once)
		expect(JSON.stringify(normalizeScheduleCfg(once))).toBe(JSON.stringify(once))
		expect(once).toMatchObject({
			schedule: '0 0 12 * * *',
			on_failure: 'script/hub/1/workspace-or-schedule-error-handler-slack',
			on_failure_extra_args: { channel: '#alerts' },
			on_recovery: undefined,
			summary: undefined
		})
		expect(once).not.toHaveProperty('edited_by')
	})

	// Deploying a draft-only schedule creates it, and a create always enables it. A draft that
	// kept its stored state would show a disabled schedule the server has enabled, and later
	// updates omit the field, so nothing would ever correct it.
	it('records a draft-only schedule as enabled, however it was stored', () => {
		expect(
			draftOnlyScheduleCfg(normalizeScheduleCfg({ ...deployed, enabled: false }))
		).toMatchObject({ enabled: true })
		const { enabled: _dropped, ...withoutEnabled } = deployed
		expect(draftOnlyScheduleCfg(normalizeScheduleCfg(withoutEnabled))).toMatchObject({
			enabled: true
		})
	})
})
