import { describe, expect, it } from 'vitest'
import { syncSlackBotArg } from './handlerSlackArg'

const AUTO = '$res:f/slack_bot/bot_token'

describe('syncSlackBotArg', () => {
	it("leaves a custom handler's slack value alone", () => {
		const args = { slack: '$res:u/me/my_slack' }
		syncSlackBotArg(args, false, false)
		syncSlackBotArg(args, false, true)
		expect(args).toEqual({ slack: '$res:u/me/my_slack' })
	})

	it('sets the bot token for the built-in and drops it when leaving', () => {
		const args: Record<string, any> = {}
		syncSlackBotArg(args, true, false)
		expect(args.slack).toBe(AUTO)
		syncSlackBotArg(args, false, true)
		expect(args).toStrictEqual({})
	})
})
