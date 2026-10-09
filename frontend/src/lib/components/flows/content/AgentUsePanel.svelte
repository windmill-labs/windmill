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
	import { generateRandomString } from '$lib/utils'

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
	let url = $derived(
		`${location.origin}${base}/api/w/${workspace}/jobs/run/agent/${agentPath}` +
			(chat ? '?memory_id=my-conversation-1' : '')
	)
	let resultUrl = $derived(
		`${location.origin}${base}/api/w/${workspace}/jobs_u/completed/get_result_maybe`
	)
	const body = JSON.stringify({ user_message: 'Hello' })
	let headers = $derived({ 'Content-Type': 'application/json', Authorization: `Bearer ${token}` })

	let curlCode = $derived(`TOKEN='${token}'
BODY='${body}'
URL='${url}'
UUID=$(curl -s -H 'Content-Type: application/json' -H "Authorization: Bearer $TOKEN" -X POST -d "$BODY" "$URL")

URL="${resultUrl}/$UUID"
while true; do
  curl -s -H "Authorization: Bearer $TOKEN" "$URL" -o res.json
  COMPLETED=$(cat res.json | jq .completed)
  if [ "$COMPLETED" = "true" ]; then
    cat res.json | jq .result.output
    break
  else
    sleep 1
  fi
done`)

	let fetchCode = $derived(`export async function main() {
  const UUID = await (await sendMessage()).text();
  return await waitForAnswer(UUID);
}

async function sendMessage() {
  return await fetch(\`${url}\`, {
    method: 'POST',
    headers: ${JSON.stringify(headers, null, 2).replaceAll('\n', '\n    ')},
    body: JSON.stringify(${body})
  });
}

async function waitForAnswer(UUID) {
  while (true) {
    const res = await fetch(\`${resultUrl}/\${UUID}\`, {
      headers: { Authorization: 'Bearer ${token}' }
    });
    const data = await res.json();
    if (data.completed) return data.result?.output;
    await new Promise((r) => setTimeout(r, 1000));
  }
}`)
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
					{#if !slackTeamName}
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
								Slack refused the workspace's bot token or couldn't be reached, so agents can't answer there.
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
					<span class="text-xs text-secondary">
						Each call runs the agent as the token's owner and returns the id of the run.{#if chat}{' '}Calls
							that share a <code>memory_id</code> are one conversation: reuse it for follow-ups and pick
							a new one to start over.{/if}
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
					<Tabs selected="rest">
						<Tab value="rest" label="REST" />
						<Tab value="curl" label="Curl" />
						<Tab value="fetch" label="Fetch" />
						{#snippet content()}
							<TabContent value="rest" class="flex flex-col gap-6 mt-2">
								<Label label="Url"><ClipboardPanel content={url} /></Label>
								<Label label="Body">
									<ClipboardPanel content={JSON.stringify(JSON.parse(body), null, 2)} />
								</Label>
								<Label label="Headers">
									<ClipboardPanel content={JSON.stringify(headers, null, 2)} />
								</Label>
								<span class="text-xs text-secondary">
									Then poll <code class="break-all">{resultUrl}/&lbrace;id&rbrace;</code> until
									<code>completed</code>
									is true; the answer is the result's <code>output</code>.
								</span>
							</TabContent>
							<TabContent value="curl" class="mt-2">
								<CopyableCodeBlock code={curlCode} language={bash} wrap />
							</TabContent>
							<TabContent value="fetch" class="mt-2">
								<CopyableCodeBlock code={fetchCode} language={typescript} wrap />
							</TabContent>
						{/snippet}
					</Tabs>
				</TabContent>
			</div>
		{/snippet}
	</Tabs>
</div>
