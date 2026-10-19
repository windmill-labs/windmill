<script lang="ts">
	import { resource } from 'runed'
	import { Badge } from '$lib/components/common'
	import SettingCard from '$lib/components/instanceSettings/SettingCard.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { SlackAgentsService } from '$lib/gen'

	/** The channels the workspace's agents are shared in, workspace-wide. */
	let { workspace }: { workspace: string } = $props()

	const channels = resource(
		() => workspace,
		async (workspace) => await SlackAgentsService.listSlackAgentChannels({ workspace })
	)
</script>

{#if channels.current?.length}
	<SettingCard
		label="Agents shared in Slack"
		description="Bot mentions that name no agent go to the channel's default agent. Share an agent or change its channels from its settings."
	>
		<DataTable>
			<Head>
				<tr>
					<Cell head first>Channel</Cell>
					<Cell head>Agent</Cell>
					<Cell head last></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface-tertiary">
				{#each channels.current as channel (channel.channel_id + channel.agent_path)}
					<Row>
						<Cell first>#{channel.channel_name}</Cell>
						<Cell><a href="/agents/get/{channel.agent_path}">{channel.agent_path}</a></Cell>
						<Cell last class="w-24">
							{#if channel.is_default}
								<Badge small>Default</Badge>
							{/if}
						</Cell>
					</Row>
				{/each}
			</tbody>
		</DataTable>
	</SettingCard>
{/if}
