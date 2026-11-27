<!--
@component
The account settings drawers, one per concern, opened by `#user-settings[-<panel>]`.
At most one is open: opening another closes the current one without clearing the hash.
-->
<script lang="ts">
	import {
		parseAccountSettingsHash,
		type AccountSettingsPanel
	} from '$lib/components/sidebar/settings'
	import ProfileDrawer from './ProfileDrawer.svelte'
	import TokensDrawer from './TokensDrawer.svelte'
	import AiSettingsDrawer from './AiSettingsDrawer.svelte'
	import McpDrawer from './McpDrawer.svelte'

	let active = $state<AccountSettingsPanel | undefined>(undefined)

	export function open(panel: AccountSettingsPanel) {
		active = panel
	}

	export function openFromHash(hash: string) {
		const panel = parseAccountSettingsHash(hash)
		if (panel) active = panel
	}

	function binding(panel: AccountSettingsPanel) {
		return {
			get: () => active === panel,
			set: (value: boolean) => {
				if (value) {
					active = panel
				} else if (active === panel) {
					active = undefined
					if (parseAccountSettingsHash(window.location.hash)) window.location.hash = ''
				}
			}
		}
	}

	const profile = binding('profile')
	const tokens = binding('tokens')
	const ai = binding('ai')
	const mcp = binding('mcp')
</script>

<ProfileDrawer bind:open={profile.get, profile.set} />
<TokensDrawer bind:open={tokens.get, tokens.set} />
<AiSettingsDrawer bind:open={ai.get, ai.set} />
<McpDrawer bind:open={mcp.get, mcp.set} />
