<script lang="ts">
	import { Copy } from 'lucide-svelte'
	import CollapseLink from '$lib/components/CollapseLink.svelte'
	import { bash, markdown, typescript } from 'svelte-highlight/languages'
	import { Alert, Badge, Button, CopyButton, Tab, TabContent, Tabs } from '$lib/components/common'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import type { SlackAgentChannel } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { copyToClipboard, generateRandomString } from '$lib/utils'
	import { apiBaseUrl } from '$lib/apiBaseUrl.svelte'
	import {
		curlExample,
		fetchExample,
		integrationPrompt,
		runUrl,
		sdkExample,
		sdkReactExample
	} from './agentApiExamples'

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
		channels
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
	} = $props()

	let pickedTab = $state('api')
	let tab = $derived(chat ? pickedTab : 'api')
	let handle = $derived(`+${agentPath.split('/').pop()}`)

	let userSettings: UserSettings | undefined = $state(undefined)
	let token = $state('')
	let target = $derived({
		api: `${apiBaseUrl()}/api/w/${workspace}`,
		agentPath,
		chat,
		token
	})
	let body = JSON.stringify({ user_message: 'Hello' }, null, 2)
	let headers = $derived(
		JSON.stringify(
			{ 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
			null,
			2
		)
	)
	let apiTab = $state('prompt')
	let tsFlavor = $state('react')
</script>

<UserSettings
	bind:this={userSettings}
	on:tokenCreated={(e) => (token = e.detail)}
	newTokenWorkspace={workspace}
	newTokenLabel={`agent-${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
	scopes={[`jobs:run:agents:${agentPath}`]}
/>

<div class="flex flex-col h-full">
	<Tabs bind:selected={() => tab, (t) => (pickedTab = t)} wrapperClass="flex-none w-full">
		<Tab value="api" label="API" />
		{#if chat}
			<Tab value="slack" label="Slack" />
		{/if}

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
								The server needs the Slack app's signing secret, so Windmill can tell who sent a
								message. An instance admin sets it as the <code>SLACK_SIGNING_SECRET</code> environment
								variable.
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

				<TabContent value="api" class="flex flex-col gap-6 p-4">
					<Label label="Token">
						<div class="flex flex-col gap-2">
							<TextInput
								bind:value={token}
								inputProps={{ placeholder: 'Paste your token here once created' }}
								class="!text-xs !font-normal"
							/>
							<div>
								<Button
									variant="default"
									unifiedSize="sm"
									onClick={() => userSettings?.openDrawer()}
								>
									Create an agent-specific token
									<Tooltip light>
										The token can only run this agent. It is safe to share as it cannot be used to
										impersonate you.
									</Tooltip>
								</Button>
							</div>
						</div>
					</Label>
					<div class="flex flex-col">
						<Tabs bind:selected={apiTab}>
							<Tab value="prompt" label="AI prompt" />
							<Tab value="ts" label="TypeScript" />
							<Tab value="http" label="HTTP" />
							{#snippet content()}
								<TabContent value="ts" class="flex flex-col gap-2 mt-2">
									{#if chat}
										<ToggleButtonGroup bind:selected={tsFlavor} noWFull>
											{#snippet children({ item })}
												<ToggleButton value="react" label="React" {item} small />
												<ToggleButton value="any" label="Any framework" {item} small />
												<ToggleButton value="fetch" label="No SDK" {item} small />
											{/snippet}
										</ToggleButtonGroup>
									{/if}
									<CopyableCodeBlock
										code={!chat || tsFlavor === 'fetch'
											? fetchExample(target)
											: tsFlavor === 'react'
												? sdkReactExample(target)
												: sdkExample(target)}
										language={typescript}
									/>
								</TabContent>
								<TabContent value="http" class="flex flex-col gap-6 mt-2">
									<Label label="Url">
										<ClipboardPanel
											content={runUrl(target) + (chat ? '?memory_id=<conversation id>' : '')}
										/>
									</Label>
									<Label label="Body"><ClipboardPanel content={body} /></Label>
									<Label label="Headers"><ClipboardPanel content={headers} /></Label>
									<Label label="Curl">
										<span class="text-xs text-secondary">
											Runs the agent and waits for its answer. The TypeScript tab streams it
											instead.
										</span>
										<CopyableCodeBlock code={curlExample(target)} language={bash} />
									</Label>
								</TabContent>
								<TabContent value="prompt" class="flex flex-col gap-2 mt-2">
									<span class="text-xs text-secondary">
										Share this with your coding assistant. It assumes your token is in <code
											>WINDMILL_TOKEN</code
										>.
									</span>
									<div>
										<Button
											variant="default"
											unifiedSize="sm"
											startIcon={{ icon: Copy }}
											onClick={() => copyToClipboard(integrationPrompt(target))}
										>
											Copy prompt
										</Button>
									</div>
									<CollapseLink text="Show prompt">
										<CopyableCodeBlock code={integrationPrompt(target)} language={markdown} wrap />
									</CollapseLink>
								</TabContent>
							{/snippet}
						</Tabs>
					</div>
				</TabContent>
			</div>
		{/snippet}
	</Tabs>
</div>
