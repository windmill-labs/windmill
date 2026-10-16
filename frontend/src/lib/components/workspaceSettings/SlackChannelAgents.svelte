<script lang="ts">
	import { resource } from 'runed'
	import { Badge } from '$lib/components/common'
	import { SlackAgentsService } from '$lib/gen'

	/** The channels the workspace's agents are shared in, workspace-wide. */
	let { workspace }: { workspace: string } = $props()

	const channels = resource(
		() => workspace,
		async (workspace) => await SlackAgentsService.listSlackAgentChannels({ workspace })
	)
</script>

{#if channels.current?.length}
	<div class="flex flex-col gap-2">
		<span class="text-xs font-semibold text-emphasis">Agents shared in Slack</span>
		<span class="text-xs text-secondary">
			Mentions of the bot and <code>/windmill</code> commands in a channel go to its default agent, not
			to the command script. Share an agent or change its channels from the agent's page.
		</span>
		<div class="flex flex-col gap-1">
			{#each channels.current as channel (channel.channel_id + channel.agent_path)}
				<div class="flex items-center justify-between gap-2 rounded border px-2 py-1 text-xs">
					<span class="text-primary">#{channel.channel_name}</span>
					<span class="flex items-center gap-2">
						{#if channel.is_default}
							<Badge small>Default</Badge>
						{/if}
						<a href="/agents/get/{channel.agent_path}">{channel.agent_path}</a>
					</span>
				</div>
			{/each}
		</div>
	</div>
{/if}
