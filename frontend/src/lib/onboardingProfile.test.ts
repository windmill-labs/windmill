import { describe, expect, it, vi } from 'vitest'

const { getOnboardingProfile } = vi.hoisted(() => ({ getOnboardingProfile: vi.fn() }))
vi.mock('./gen', () => ({ UserService: { getOnboardingProfile } }))

import { noteSessionEmail, onboardingProfile, parseOnboardingProfile } from './onboardingProfile'

describe('onboardingProfile', () => {
	it('serves each session its own profile, never the previous one', async () => {
		getOnboardingProfile
			.mockResolvedValueOnce({ profile: { workspace_name: 'Acme' } })
			.mockResolvedValueOnce({ profile: { workspace_name: 'Globex' } })

		noteSessionEmail('a@example.com')
		expect((await onboardingProfile())?.workspace_name).toBe('Acme')
		expect((await onboardingProfile())?.workspace_name).toBe('Acme')
		expect(getOnboardingProfile).toHaveBeenCalledTimes(1)

		noteSessionEmail('b@example.com')
		expect((await onboardingProfile())?.workspace_name).toBe('Globex')
		expect(getOnboardingProfile).toHaveBeenCalledTimes(2)
	})
})

describe('parseOnboardingProfile', () => {
	it('keeps the well-formed keys when another one is malformed', () => {
		expect(
			parseOnboardingProfile({
				touch_point: 'outbound:q3',
				workspace_name: '  Acme  ',
				hub_projects: ['uptime-monitor', 42, ''],
				starter_prompts: [
					{ label: 'Sync', prompt: 'Sync HubSpot to Postgres' },
					{ label: 'no prompt' },
					{ label: 'Sync', prompt: 'a second prompt under the same label' }
				],
				tools: ['HubSpot']
			})
		).toEqual({
			touch_point: 'outbound:q3',
			company: undefined,
			workspace_name: 'Acme',
			hub_projects: ['uptime-monitor'],
			starter_prompts: [{ label: 'Sync', prompt: 'Sync HubSpot to Postgres' }],
			tools: ['hubspot']
		})
	})

	it('is null for an empty, non-object, or entirely unusable profile', () => {
		expect(parseOnboardingProfile(null)).toBeNull()
		expect(parseOnboardingProfile([])).toBeNull()
		expect(parseOnboardingProfile({ starter_prompts: 'not a list', hub_projects: [] })).toBeNull()
	})
})
