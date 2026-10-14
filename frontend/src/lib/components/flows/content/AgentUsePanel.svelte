<script lang="ts">
	import { Copy, Plus, X } from 'lucide-svelte'
	import CollapseLink from '$lib/components/CollapseLink.svelte'
	import { resource } from 'runed'
	import { bash, markdown, typescript } from 'svelte-highlight/languages'
	import { Alert, Button, CopyButton, Tab, TabContent, Tabs } from '$lib/components/common'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import { SlackAgentsService, type SlackChannelAgent } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
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
	 * The ways to use an agent outside its page: in Slack, anywhere by its `+name` and without one
	 * in the channels it is the default of, and over the API. Only admins change the channels, as
	 * only they change the workspace's Slack settings.
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
		channels,
		onChanged
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
		channels: SlackChannelAgent[]
		onChanged: () => void
	} = $props()

	let pickedTab = $state('api')
	let tab = $derived(chat ? pickedTab : 'api')
	let handle = $derived(`+${agentPath.split('/').pop()}`)

	type Choice = { label: string; value: string; name: string }
	let adding = $state(false)
	let saving = $state(false)

	// Read again whenever the agent's channels change, so a channel just added or removed moves
	// between the list and the picker.
	const available = resource(
		() => ({
			ws:
				isAdmin && chat && slackTeamName && botName && missingScopes.length === 0
					? workspace
					: undefined,
			channels
		}),
		async ({ ws }) => (ws ? await listChoices(ws) : [])
	)

	async function listChoices(ws: string): Promise<Choice[]> {
		try {
			// Which agent each channel already has, so picking one says what it replaces.
			const taken = new Map(
				(await SlackAgentsService.listSlackChannelAgents({ workspace: ws })).map((c) => [
					c.channel_id,
					c.agent_path
				])
			)
			const all: Choice[] = []
			let cursor: string | undefined = undefined
			// ponytail: stops at 10 pages of 200 channels; search on the server if a team has more.
			for (let page = 0; page < 10; page++) {
				const res = await SlackAgentsService.listAvailableSlackChannels({ workspace: ws, cursor })
				for (const c of res.channels) {
					const current = taken.get(c.id)
					if (current === agentPath) continue
					all.push({
						value: c.id,
						name: c.name,
						label: current ? `#${c.name} (replaces ${current})` : `#${c.name}`
					})
				}
				cursor = res.next_cursor
				if (!cursor) break
			}
			return all
		} catch (e) {
			sendUserToast(`Couldn't list the Slack channels: ${e.body ?? e.message}`, true)
			return []
		}
	}

	async function add(channelId: string | undefined) {
		const choice = available.current?.find((c) => c.value === channelId)
		if (!choice) return
		saving = true
		try {
			await SlackAgentsService.setSlackChannelAgent({
				workspace,
				requestBody: { channel_id: choice.value, channel_name: choice.name, agent_path: agentPath }
			})
			adding = false
			onChanged()
		} catch (e) {
			sendUserToast(e.body ?? e.message, true)
		} finally {
			saving = false
		}
	}

	async function remove(channel: SlackChannelAgent) {
		try {
			await SlackAgentsService.removeSlackChannelAgent({
				workspace,
				channelId: channel.channel_id
			})
			onChanged()
		} catch (e) {
			sendUserToast(e.body ?? e.message, true)
		}
	}

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
								The connection lacks {missingScopes.join(', ')}, so agents can't tell who is asking.
								{#if isAdmin}
									<a href="/workspace_settings?tab=slack&agents=true">Disconnect Slack</a>, then
									connect it again with AI agents enabled. Disconnecting clears the channel
									defaults.
								{:else}
									A workspace admin can reconnect Slack with AI agents enabled.
								{/if}
							</Alert>
						{/if}
						<div class="flex flex-col gap-1">
							<span class="text-xs font-semibold text-emphasis">Ask it anywhere</span>
							<span class="text-xs text-secondary">
								In {slackTeamName}, mention the bot and start your message with the agent's name, in
								a channel or in a direct message. Follow-ups in the same thread that mention the bot
								go to the same agent.
							</span>
							<div class="flex items-center gap-2 mt-1">
								<code class="text-xs bg-surface-secondary rounded px-2 py-1"
									>{botName ? `@${botName} ` : ''}{handle} your question</code
								>
								<CopyButton value={handle} title="Copy {handle}" />
								<Tooltip>
									{handle} matches the last part of the agent's path. Use the whole path,
									<code>+{agentPath}</code>, when two agents share that name.
								</Tooltip>
							</div>
						</div>

						<div class="flex flex-col gap-2">
							<span class="text-xs font-semibold text-emphasis">Default in these channels</span>
							<span class="text-xs text-secondary">
								Mentions of the bot and <code>/windmill</code> commands in these channels go to this
								agent without its name. They no longer reach the workspace's Slack command script there.
							</span>
							{#if channels.length === 0 && !adding}
								<span class="text-xs text-hint">No channel yet.</span>
							{/if}
							{#each channels as channel (channel.channel_id)}
								<div class="flex items-center gap-2 w-full">
									<TextInput
										class="flex-1"
										inputProps={{ readonly: true }}
										value="#{channel.channel_name}"
									/>
									{#if isAdmin}
										<Button
											variant="subtle"
											unifiedSize="md"
											iconOnly
											destructive
											startIcon={{ icon: X }}
											title="Remove #{channel.channel_name}"
											onClick={() => remove(channel)}
										/>
									{/if}
								</div>
							{/each}
							{#if isAdmin && missingScopes.length === 0}
								{#if adding}
									<div class="flex items-center gap-2 w-full">
										<Select
											class="flex-1"
											items={available.current ?? []}
											bind:value={() => undefined, (v) => add(v)}
											loading={available.loading || saving}
											disabled={saving}
											placeholder="Pick a public channel"
											noItemsMsg="No other public channel"
											open
										/>
										<Button
											variant="subtle"
											unifiedSize="md"
											iconOnly
											startIcon={{ icon: X }}
											title="Cancel"
											onClick={() => (adding = false)}
										/>
									</div>
								{:else}
									<div class="flex justify-start">
										<Button
											variant="default"
											unifiedSize="sm"
											startIcon={{ icon: Plus }}
											onClick={() => (adding = true)}
										>
											Add channel
										</Button>
									</div>
								{/if}
							{:else if !isAdmin}
								<span class="text-2xs text-hint">Only workspace admins can change these.</span>
							{/if}
						</div>

						<Alert type="info" title="Each person runs the agent as themselves" size="xs">
							Their Slack email must belong to a member of this workspace, and the agent can use
							only what that member can.
						</Alert>
					{/if}
				</TabContent>

				<TabContent value="api" class="flex flex-col gap-6 p-4">
					<span class="text-xs text-secondary">
						Each call runs the agent as the token's owner and returns the id of the run, whose
						answer streams as it is written.{#if chat}{' '}Calls that share a
							<code>memory_id</code>, any string you choose, are one conversation.{/if}
					</span>
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
										For your coding assistant to write the integration: the endpoints and streaming{chat
											? ', conversations and the chat SDK'
											: ''}. The token stays a placeholder.
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
