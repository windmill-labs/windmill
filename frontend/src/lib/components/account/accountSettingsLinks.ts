import { KeyRound, Plug, Sparkles, UserCog } from 'lucide-svelte'
import { accountSettingsHref, type AccountSettingsPanel } from '$lib/components/sidebar/settings'

export type AccountSettingsLink = {
	panel: AccountSettingsPanel
	label: string
	description: string
	href: string
	icon: typeof UserCog
}

function link(
	panel: AccountSettingsPanel,
	label: string,
	description: string,
	icon: typeof UserCog
): AccountSettingsLink {
	return { panel, label, description, href: accountSettingsHref(panel), icon }
}

/** The account settings drawers, in the order every menu lists them. */
export const ACCOUNT_SETTINGS_LINKS: AccountSettingsLink[] = [
	link('profile', 'Profile', 'Email, password and theme', UserCog),
	link('tokens', 'Tokens', 'Personal access tokens for the API and the CLI', KeyRound),
	link('ai', 'AI settings', 'Completions, custom prompts and your AI usage', Sparkles),
	link('mcp', 'Connect an AI client', 'Use Windmill from Claude, ChatGPT or Cursor over MCP', Plug)
]
