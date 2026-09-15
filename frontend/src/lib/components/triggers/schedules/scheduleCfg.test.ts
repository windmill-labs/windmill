import { describe, it, expect } from 'vitest'
import {
	normalizeScheduleCfg,
	scheduleCfgAfterWrite,
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

	// Neither endpoint takes `enabled` from the config, and the item records this as its deployed
	// side: a disabled config recorded as sent would show a running schedule as disabled for good.
	it('records the enabled state a write leaves on the server, not the one sent', () => {
		const disabled = normalizeScheduleCfg({ ...deployed, enabled: false })
		const created = scheduleCfgAfterWrite(disabled, false, undefined)
		// Serialized like the form's own config, since saves compare them exactly.
		expect(JSON.stringify(created)).toBe(JSON.stringify(normalizeScheduleCfg(deployed)))
		const { enabled: _dropped, ...withoutEnabled } = deployed
		expect(
			scheduleCfgAfterWrite(normalizeScheduleCfg(withoutEnabled), false, undefined)
		).toMatchObject({ enabled: true })
		expect(scheduleCfgAfterWrite(disabled, true, normalizeScheduleCfg(deployed))).toMatchObject({
			enabled: true
		})
	})
})
