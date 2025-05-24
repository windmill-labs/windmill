<script lang="ts">
	import { AgentKnownSuffix, concatAgentSuffix, removeAgentSuffix } from '$lib/utils'
	import Section from './Section.svelte'
	import Toggle from './Toggle.svelte'

	type Props = {
		suffix: string | undefined
	}

	let enable_ssh_repl_like = $state(false)

	let { suffix = $bindable() }: Props = $props()
</script>

<Section label="Extra behavior">
	<Toggle
		textClass="font-normal text-sm"
		color="nord"
		size="xs"
		checked={enable_ssh_repl_like}
		on:change={() => {
			enable_ssh_repl_like = !enable_ssh_repl_like
			if (enable_ssh_repl_like) {
				suffix = concatAgentSuffix(suffix, AgentKnownSuffix.ENABLE_LIVE_SHELL)
				return
			}
			suffix = removeAgentSuffix(suffix, AgentKnownSuffix.ENABLE_LIVE_SHELL)
		}}
		options={{
			right: "Enable live shell on worker's host machine",
			rightTooltip:
				"Allow you to open a live shell and interact with the agent worker's host machine"
		}}
		class="py-1"
	/>
</Section>
