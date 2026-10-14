import { describe, expect, it } from 'vitest'
import {
	handlerDraftFromStored,
	handlerDraftToStored,
	handlerTypeForPath
} from './handlerDraft.svelte'

const SLACK_BOT = '$res:f/slack_bot/bot_token'

describe('handler draft', () => {
	it('round-trips every destination unchanged', () => {
		const stored = [
			{
				path: 'hub/28794/workspace-or-schedule-error-handler-slack',
				kind: 'script' as const,
				args: { channel: 'alerts', slack: SLACK_BOT }
			},
			{
				path: 'hub/19742/workspace-or-schedule-error-handler-teams',
				kind: 'script' as const,
				args: { channel: 'c1', channel_name: 'General' }
			},
			{
				path: 'hub/19795/workspace-or-error-handler-email',
				kind: 'script' as const,
				args: { email_recipients: ['ops@example.com'] }
			},
			{
				path: 'f/handlers/on_error',
				kind: 'flow' as const,
				args: { slack: '$res:u/me/my_slack', channel: 'own' }
			}
		]
		for (const s of stored) {
			const draft = handlerDraftFromStored('error', s.path, s.kind, s.args)
			expect(handlerDraftToStored(draft)).toEqual(s)
		}
	})

	it("saves only the selected destination's settings", () => {
		const draft = handlerDraftFromStored('error', 'u/me/handler', 'script', {
			webhook_url: 'https://hooks.example/custom'
		})
		draft.selected = 'email'
		draft.email.args.email_recipients = ['ops@example.com']
		expect(handlerDraftToStored(draft)).toEqual({
			path: 'hub/19795/workspace-or-error-handler-email',
			kind: 'script',
			args: { email_recipients: ['ops@example.com'] }
		})
	})

	it('recognises built-in handlers per mode', () => {
		const recoverySlack = 'hub/28791/slack/schedule-recovery-handler-slack'
		expect(handlerTypeForPath('recovery', recoverySlack)).toBe('slack')
		expect(handlerTypeForPath('error', recoverySlack)).toBe('custom')
	})

	it('stores no handler when the destination has none', () => {
		const draft = handlerDraftFromStored('error', undefined)
		expect(handlerDraftToStored(draft)).toEqual({ kind: 'script', args: {} })
		draft.selected = 'slack'
		expect(handlerDraftToStored(draft)).toEqual({ kind: 'script', args: {} })
		draft.selected = 'instance_alerts'
		expect(handlerDraftToStored(draft)).toEqual({ kind: 'script', args: {} })
	})
})
