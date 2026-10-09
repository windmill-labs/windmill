<script lang="ts">
	import { X } from 'lucide-svelte'
	import { resource } from 'runed'
	import { bash, typescript } from 'svelte-highlight/languages'
	import { base } from '$lib/base'
	import { Alert, Button, CopyButton, Tab, TabContent, Tabs } from '$lib/components/common'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import { SlackAgentsService, type SlackChannelAgent } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard, generateRandomString } from '$lib/utils'
	import { Sparkles } from 'lucide-svelte'
	import {
		curlExample,
		fetchExample,
		integrationPrompt,
		runUrl,
		sdkExample
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
		/** Agents answer in Slack only when the instance verifies Slack's request signatures. */
		signingSecretSet: boolean
		channels: SlackChannelAgent[]
		onChanged: () => void
	} = $props()

	let pickedTab = $state('slack')
	let tab = $derived(chat ? pickedTab : 'api')
	let handle = $derived(`+${agentPath.split('/').pop()}`)

	type Choice = { label: string; value: string; name: string }
	let picked = $state<string | undefined>(undefined)
	let saving = $state(false)

	// Read again whenever the agent's channels change, so a channel just added or removed moves
	// between the list and the picker.
	const available = resource(
		() => ({ ws: isAdmin && chat && slackTeamName && botName ? workspace : undefined, channels }),
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

	async function add() {
		const choice = available.current?.find((c) => c.value === picked)
		if (!choice) return
		saving = true
		try {
			await SlackAgentsService.setSlackChannelAgent({
				workspace,
				requestBody: { channel_id: choice.value, channel_name: choice.name, agent_path: agentPath }
			})
			picked = undefined
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
		api: `${location.origin}${base}/api/w/${workspace}`,
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
	let apiTab = $state<string | undefined>(undefined)
	let shownApiTab = $derived(
		apiTab === 'sdk' && !chat ? 'rest' : (apiTab ?? (chat ? 'sdk' : 'rest'))
	)
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
		{#if chat}
			<Tab value="slack" label="Slack" />
		{/if}
		<Tab value="api" label="API" />

		{#snippet content()}
			<div class="min-h-0 grow overflow-y-auto">
				<TabContent value="slack" class="flex flex-col gap-6 p-4">
					{#if slackLoading}
						<span class="text-xs text-hint">Loading…</span>
					{:else if !slackTeamName}
						<span class="text-xs text-secondary">
							Connect Slack to the workspace to ask this agent from Slack, by its name or as a
							channel's default.
						</span>
						{#if isAdmin}
							<div>
								<Button
									variant="default"
									unifiedSize="md"
									href="{base}/workspace_settings?tab=slack"
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
								The instance needs SLACK_SIGNING_SECRET set to the Slack app's signing secret, so
								Windmill can tell who sent a message.
							</Alert>
						{/if}
						{#if !botName}
							<Alert type="warning" title="Windmill can't reach Slack" size="xs">
								Slack refused the workspace's bot token or couldn't be reached, so agents can't
								answer there.
								{#if isAdmin}
									<a href="{base}/workspace_settings?tab=slack">Reconnect Slack</a> to fix it.
								{:else}
									A workspace admin can reconnect Slack to fix it.
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
								Mentions of the bot in these channels go to this agent without its name.
							</span>
							{#if channels.length === 0}
								<span class="text-xs text-hint">No channel yet.</span>
							{:else}
								<div class="flex flex-col gap-1">
									{#each channels as channel (channel.channel_id)}
										<div class="flex items-center justify-between rounded border px-2 py-1">
											<span class="text-xs text-primary">#{channel.channel_name}</span>
											{#if isAdmin}
												<Button
													variant="subtle"
													unifiedSize="xs"
													iconOnly
													startIcon={{ icon: X }}
													title="Remove #{channel.channel_name}"
													onClick={() => remove(channel)}
												/>
											{/if}
										</div>
									{/each}
								</div>
							{/if}
							{#if isAdmin}
								<div class="flex items-center gap-2">
									<Select
										class="grow"
										items={available.current ?? []}
										bind:value={picked}
										loading={available.loading}
										placeholder="Pick a public channel"
										noItemsMsg="No other public channel"
									/>
									<Button
										variant="default"
										unifiedSize="md"
										disabled={!picked || saving}
										loading={saving}
										onClick={add}
									>
										Add
									</Button>
								</div>
							{:else}
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
					<div class="flex flex-col gap-2">
						<span class="text-xs text-secondary">
							Each call runs the agent as the token's owner and returns the id of the run, whose
							answer streams as it is written.{#if chat}{' '}Calls that share a
								<code>memory_id</code>, any string you choose, are one conversation.{/if}
						</span>
						<div>
							<Button
								variant="default"
								unifiedSize="sm"
								startIcon={{ icon: Sparkles }}
								onClick={() => copyToClipboard(integrationPrompt(target))}
							>
								Copy a prompt for your AI assistant
								<Tooltip light>
									Everything a coding assistant needs to integrate this agent: the endpoints,
									streaming{chat ? ', conversations and the chat library' : ''}. The token is left
									as a placeholder.
								</Tooltip>
							</Button>
						</div>
					</div>
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
					<Tabs bind:selected={() => shownApiTab, (t) => (apiTab = t)}>
						{#if chat}
							<Tab value="sdk" label="Chat SDK" />
						{/if}
						<Tab value="rest" label="REST" />
						<Tab value="curl" label="Curl" />
						<Tab value="fetch" label="Fetch" />
						{#snippet content()}
							<TabContent value="sdk" class="flex flex-col gap-2 mt-2">
								<span class="text-xs text-secondary">
									React with <code>windmill-chat</code>: streaming, tool calls and conversations
									handled. It also plugs into the Vercel AI SDK and assistant-ui.
								</span>
								<CopyableCodeBlock code={sdkExample(target)} language={typescript} />
							</TabContent>
							<TabContent value="rest" class="flex flex-col gap-6 mt-2">
								<Label label="Url">
									<ClipboardPanel
										content={runUrl(target) + (chat ? '?memory_id=<conversation id>' : '')}
									/>
								</Label>
								<Label label="Body"><ClipboardPanel content={body} /></Label>
								<Label label="Headers"><ClipboardPanel content={headers} /></Label>
								<span class="text-xs text-secondary">
									Then follow the run: the Fetch example streams it, the Curl one waits for the
									answer.
								</span>
							</TabContent>
							<TabContent value="curl" class="mt-2">
								<CopyableCodeBlock code={curlExample(target)} language={bash} />
							</TabContent>
							<TabContent value="fetch" class="mt-2">
								<CopyableCodeBlock code={fetchExample(target)} language={typescript} />
							</TabContent>
						{/snippet}
					</Tabs>
				</TabContent>
			</div>
		{/snippet}
	</Tabs>
</div>
