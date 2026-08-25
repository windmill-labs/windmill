import { BROWSER } from 'esm-env'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'

const EXPAND_THINKING_SETTING = 'ai-chat-expand-thinking'

// How the reader wants to read, not anything about the conversation — so it
// lives per browser rather than in chat history or workspace settings, and
// applies to every chat at once.
let expandByDefault = $state(BROWSER && getLocalSetting(EXPAND_THINKING_SETTING) === 'true')

export const thinkingPreferences = {
	get expandByDefault() {
		return expandByDefault
	},
	set expandByDefault(value: boolean) {
		expandByDefault = value
		storeLocalSetting(EXPAND_THINKING_SETTING, value ? 'true' : undefined)
	}
}
