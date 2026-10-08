<script lang="ts">
	import { X } from 'lucide-svelte'
	import { resource } from 'runed'
	import { shell } from 'svelte-highlight/languages'
	import { base } from '$lib/base'
	import { Alert, Button, CopyButton, Tab, TabContent, Tabs } from '$lib/components/common'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { SlackAgentsService, type SlackChannelAgent } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	/**
	 * The ways to use an agent outside its page: in Slack, anywhere by its `+name` and without one
	 * in the channels it is the default of, and over the API. Only admins change the channels, as
	 * only they change the workspace's Slack settings.
	 */
	let {
		agentPath,
		workspace,
		isAdmin,
		slackTeamName,
		signingSecretSet,
		channels,
		onChanged
	}: {
		agentPath: string
		workspace: string
		isAdmin: boolean
		/** Unset when Slack is not connected to the workspace. */
		slackTeamName: string | undefined
		/** Agents answer in Slack only when the instance verifies Slack's request signatures. */
		signingSecretSet: boolean
		channels: SlackChannelAgent[]
		onChanged: () => void
	} = $props()

	let tab = $state('slack')
	let handle = $derived(`+${agentPath.split('/').pop()}`)

	type Choice = { label: string; value: string; name: string }
	let picked = $state<string | undefined>(undefined)
	let saving = $state(false)

	// Read again whenever the agent's channels change, so a channel just added or removed moves
	// between the list and the picker.
	const available = resource(
		() => ({ ws: isAdmin && slackTeamName ? workspace : undefined, channels }),
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

	let apiBase = $derived(`${location.origin}${base}/api/w/${workspace}`)
	let runSnippet = $derived(
		`curl -X POST "${apiBase}/jobs/run/agent/${agentPath}?memory_id=my-conversation-1" \\
  -H "Authorization: Bearer $WM_TOKEN" \\
  -H "Content-Type: application/json" \\
  -d '{"user_message": "Hello"}'`
	)
	let resultSnippet = $derived(
		`curl "${apiBase}/jobs_u/completed/get_result_maybe/$JOB_ID" \\
  -H "Authorization: Bearer $WM_TOKEN"`
	)
</script>

<div class="flex flex-col h-full">
	<Tabs bind:selected={tab} wrapperClass="flex-none w-full">
		<Tab value="slack" label="Slack" />
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
						<div class="flex flex-col gap-1">
							<span class="text-xs font-semibold text-emphasis">Ask it anywhere</span>
							<span class="text-xs text-secondary">
								In {slackTeamName}, mention the Windmill bot and start your message with the agent's
								name, in a channel or in a direct message. Follow-ups in the same thread go to the
								same agent.
							</span>
							<div class="flex items-center gap-2 mt-1">
								<code class="text-xs bg-surface-secondary rounded px-2 py-1"
									>{handle} your question</code
								>
								<CopyButton value={handle} title="Copy {handle}" />
							</div>
						</div>

						<div class="flex flex-col gap-2">
							<span class="text-xs font-semibold text-emphasis">Default in these channels</span>
							<span class="text-xs text-secondary">
								Mentions of the Windmill bot in these channels go to this agent without its name.
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
						<span class="text-xs font-semibold text-emphasis">Send a message</span>
						<span class="text-xs text-secondary">
							Returns the id of the run. Runs that share a <code>memory_id</code> are one conversation,
							so reuse it for follow-ups and pick a new one to start over. The run is the token owner's,
							with their permissions.
						</span>
						<CopyableCodeBlock code={runSnippet} language={shell} wrap />
					</div>
					<div class="flex flex-col gap-2">
						<span class="text-xs font-semibold text-emphasis">Read the answer</span>
						<span class="text-xs text-secondary">
							Poll until <code>completed</code> is true; the answer is the result's
							<code>output</code>.
						</span>
						<CopyableCodeBlock code={resultSnippet} language={shell} wrap />
					</div>
					<span class="text-2xs text-hint">
						Create a token in your <a href="#user-settings">account settings</a>.
					</span>
				</TabContent>
			</div>
		{/snippet}
	</Tabs>
</div>
