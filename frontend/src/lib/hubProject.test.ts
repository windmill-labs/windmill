import { describe, expect, it, vi } from 'vitest'

vi.mock('./gen', () => ({ HubPublishService: {}, SettingService: {} }))
vi.mock('./components/icons', () => ({ appIconComponent: () => undefined }))

import { hubProjectDescription } from './hubProject'

describe('hubProjectDescription', () => {
	it('prefers the description field when the hub has one', () => {
		expect(hubProjectDescription({ description: '  Runs payroll.  ', readme: '# Other' })).toBe(
			'Runs payroll.'
		)
	})

	it('reads the readme intro when it does not, which is every published project', () => {
		expect(
			hubProjectDescription({
				description: '',
				readme: 'Audiences and campaigns,\nwith a sending engine.\n\n## Concepts\n\n- A flow'
			})
		).toBe('Audiences and campaigns, with a sending engine.')
	})

	it('skips a leading heading rather than stopping at it', () => {
		expect(
			hubProjectDescription({
				readme: '## Description\n\nManages Odoo records.\n\n## Usage\n\n1. Generate a key'
			})
		).toBe('Manages Odoo records.')
	})

	it('strips inline markdown', () => {
		expect(
			hubProjectDescription({ readme: 'A **bold** clone of [Bitly](https://bitly.com) with `js`.' })
		).toBe('A bold clone of Bitly with js.')
	})

	it('cuts on a word boundary, so a long one reads as shortened not corrupted', () => {
		const long = hubProjectDescription({ readme: 'lorem ipsum '.repeat(40) })
		expect(long.length).toBeLessThanOrEqual(321)
		expect(long.endsWith('…')).toBe(true)
		expect(long).not.toMatch(/lore…$/)
	})

	it('falls back to the summary when there is no prose at all', () => {
		expect(hubProjectDescription({ readme: '## Usage\n', summary: 'Short links' })).toBe(
			'Short links'
		)
		expect(hubProjectDescription({})).toBe('')
	})
})
