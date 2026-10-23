<script lang="ts">
	import { Alert, Badge, Button, CopyButton, TabContent, Tabs } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { SlackAgentChannel } from '$lib/gen'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { createRawAppFromAgent } from '$lib/components/details/createRawAppFromScript'
	import { goto } from '$app/navigation'
	import WebhooksPanel from '$lib/components/triggers/webhook/WebhooksPanel.svelte'

	/**
	 * The ways to use an agent outside its page: in the Slack channels it is shared in, set in its
	 * settings and summed up here, and over the API.
	 */
	let {
		agentPath,
		workspace,
		isAdmin,
		chat,
		slackLoading,
		slackTeamName,
		botName,
		missingScopes,
		signingSecretSet,
		runsAs,
		anyone,
		canShare,
		channels,
		section
	}: {
		agentPath: string
		workspace: string
		isAdmin: boolean
		/** Whether the agent keeps the conversation. Slack is offered only then: a thread is one. */
		chat: boolean
		/** Whether the workspace's Slack connection is still being read. */
		slackLoading: boolean
		/** Unset when Slack is not connected to the workspace. */
		slackTeamName: string | undefined
		/** Unset when Slack can't be reached with the workspace's bot token. */
		botName: string | undefined
		/** Agent scopes the connection lacks: Slack was connected without AI agents enabled. */
		missingScopes: string[]
		/** Agents answer in Slack only when the instance verifies Slack's request signatures. */
		signingSecretSet: boolean
		/** Who the agent runs as; unset unless it runs on behalf of someone. */
		runsAs: string | undefined
		/** Whether anyone in its channels can ask it, rather than workspace members only. */
		anyone: boolean
		/** Whether the user can change its Slack settings: anyone who can edit it. */
		canShare: boolean
		channels: SlackAgentChannel[]
		/** The section shown: the page offers them as its tabs. */
		section: 'api' | 'slack' | 'app'
	} = $props()

	let handle = $derived(`+${agentPath.split('/').pop()}`)

	const editRights = useEditRights(() => workspace)
	async function buildApp() {
		// The raw app editor reads the payload from sessionStorage on mount.
		sessionStorage.setItem('rawAppImport', JSON.stringify(createRawAppFromAgent(agentPath)))
		await goto('/apps_raw/add')
	}
</script>

<div class="flex flex-col h-full">
	<Tabs selected={section} hideTabs wrapperClass="flex-none w-full">
		{#snippet content()}
			<div class="min-h-0 grow overflow-y-auto">
				<TabContent value="slack" class="flex flex-col gap-6 p-4">
					{#if slackLoading}
						<span class="text-xs text-hint">Loading…</span>
					{:else if !slackTeamName}
						<span class="text-xs text-secondary">
							Connect Slack to the workspace, with AI agents enabled, to ask this agent from Slack,
							by its name or as a channel's default.
						</span>
						{#if isAdmin}
							<div>
								<Button
									variant="default"
									unifiedSize="md"
									href="/workspace_settings?tab=slack&agents=true"
								>
									Connect Slack
								</Button>
							</div>
						{:else}
							<span class="text-2xs text-hint">A workspace admin can connect it.</span>
						{/if}
					{:else}
						{#if !signingSecretSet}
							<Alert type="warning" title="Agents can't answer in Slack yet" size="xs">
								Windmill needs the Slack app's signing secret to tell who sent a message.
								{#if isAdmin}
									A workspace's own Slack app takes it in the workspace settings, the instance's app
									in the instance settings.
								{:else}
									A workspace or instance admin can set it.
								{/if}
							</Alert>
						{/if}
						{#if !botName}
							<Alert type="warning" title="Windmill can't reach Slack" size="xs">
								Slack refused the workspace's bot token or couldn't be reached, so agents can't
								answer there.
								{#if isAdmin}
									<a href="/workspace_settings?tab=slack&agents=true">Reconnect Slack</a> to fix it.
								{:else}
									A workspace admin can reconnect Slack to fix it.
								{/if}
							</Alert>
						{/if}
						{#if botName && missingScopes.length > 0}
							<Alert type="warning" title="Slack was connected without AI agents" size="xs">
								The connection lacks {missingScopes.join(', ')}, so agents can't check who is
								asking.
								{#if isAdmin}
									<a href="/workspace_settings?tab=slack&agents=true">Disconnect Slack</a>, then
									connect it again with AI agents enabled.
								{:else}
									A workspace admin can reconnect Slack with AI agents enabled.
								{/if}
							</Alert>
						{/if}

						<div class="flex flex-col gap-2">
							<div class="flex items-center justify-between gap-2">
								<span class="text-xs font-semibold text-emphasis">Ask it in Slack</span>
								{#if canShare}
									<Button
										variant="default"
										unifiedSize="sm"
										href="/agents/edit/{agentPath}?settings=true"
									>
										Edit in settings
									</Button>
								{/if}
							</div>
							{#if channels.length === 0}
								<span class="text-2xs text-hint">Not shared in any channel of {slackTeamName}.</span
								>
							{:else}
								<div class="flex items-center gap-2">
									<code
										class="text-2xs font-mono text-emphasis bg-surface-secondary rounded px-2 py-1"
										>@{botName ?? '<Slack app name>'} {handle} your question</code
									>
									<CopyButton value={handle} title="Copy {handle}" />
									<Tooltip>
										{handle} matches the last part of the agent's path. Use the whole path,
										<code>+{agentPath}</code>, when two agents shared in a channel have that name.
										Replies in the thread that mention the bot go to the same agent, no name needed.
									</Tooltip>
								</div>
								<div class="flex flex-wrap items-center gap-1">
									<span class="text-2xs text-hint">in</span>
									{#each channels as channel (channel.channel_id)}
										<Badge small
											>#{channel.channel_name}{channel.is_default ? ' · default' : ''}</Badge
										>
									{/each}
								</div>
								<span class="text-2xs text-hint">
									{#if anyone}
										Anyone in these channels can ask it, with {runsAs}'s permissions.
									{:else if runsAs}
										Workspace members can ask it, with {runsAs}'s permissions.
									{:else}
										Workspace members can ask it, with their own permissions.
									{/if}
								</span>
							{/if}
						</div>
					{/if}
				</TabContent>

				<TabContent value="app" class="flex flex-col gap-4 p-4">
					<span class="text-xs text-secondary">
						A chat app for your team, built on Windmill's chat SDK. It runs the agent as the app, so
						people need no access to the agent, and keeps their conversations in their browser. You
						can shape it like any other app.
					</span>
					<div class="flex flex-col items-start gap-1">
						<Button
							unifiedSize="sm"
							variant="default"
							disabled={!editRights.roleCanAuthor('raw_app')}
							onClick={buildApp}
						>
							Build an app for this agent
						</Button>
						{#if !editRights.roleCanAuthor('raw_app')}
							<span class="text-2xs text-hint">Operators cannot build apps in this workspace.</span>
						{/if}
					</div>
				</TabContent>

				<TabContent value="api" class="p-4">
					<WebhooksPanel
						agent
						{chat}
						path={agentPath}
						token=""
						args={{ user_message: 'Hello' }}
						scopes={[`jobs:run:agents:${agentPath}`]}
					/>
				</TabContent>
			</div>
		{/snippet}
	</Tabs>
</div>
