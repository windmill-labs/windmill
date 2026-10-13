<script lang="ts">
	import { resource } from 'runed'
	import { SlackAgentsService } from '$lib/gen'

	/** The channels whose bot mentions and `/windmill` commands an agent answers, workspace-wide. */
	let { workspace }: { workspace: string } = $props()

	const channels = resource(
		() => workspace,
		async (workspace) => await SlackAgentsService.listSlackChannelAgents({ workspace })
	)
</script>

{#if channels.current?.length}
	<div class="flex flex-col gap-2">
		<span class="text-xs font-semibold text-emphasis">Channels answered by an agent</span>
		<span class="text-xs text-secondary">
			Mentions of the bot and <code>/windmill</code> commands in these channels go to their agent, not
			to the command script. Change a channel's agent from the agent's page.
		</span>
		<div class="flex flex-col gap-1">
			{#each channels.current as channel (channel.channel_id)}
				<div class="flex items-center justify-between rounded border px-2 py-1 text-xs">
					<span class="text-primary">#{channel.channel_name}</span>
					<a href="/agents/get/{channel.agent_path}">{channel.agent_path}</a>
				</div>
			{/each}
		</div>
	</div>
{/if}
