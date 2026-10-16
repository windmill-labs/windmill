<script lang="ts">
	import { Plus, X } from 'lucide-svelte'
	import { resource } from 'runed'
	import { Button } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Label from '$lib/components/Label.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SlackAgentsService, type SlackAgentChannel } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import type { AgentDraftHandle } from '../agentDraft.svelte'

	/** Who can ask the agent in Slack, saved with it, and the channels it is shared in, which take
	 *  effect at once: they belong to one Slack workspace, not to the agent's definition. */
	interface Props {
		draft: AgentDraftHandle
		/** The deployed path; channels are shared for an agent that exists. */
		path: string
		workspace: string | undefined
	}

	let { draft, path, workspace }: Props = $props()

	let readOnly = $derived(!draft.canWrite)
	let isAdmin = $derived(!!$userStore?.is_admin)

	const slack = resource(
		() => ({ ws: workspace, path, deployed: !draft.noDeployed }),
		async ({ ws, path, deployed }) =>
			ws && deployed
				? await SlackAgentsService.getAgentSlackChannels({ workspace: ws, path })
				: undefined
	)
	let channels = $derived(slack.current?.channels ?? [])
	let canPick = $derived(
		!readOnly &&
			!!slack.current?.slack_team_name &&
			!!slack.current?.bot_name &&
			slack.current.missing_scopes.length === 0
	)

	type Choice = { label: string; value: string; name: string }
	let adding = $state(false)
	let saving = $state(false)

	// Read again whenever the channels change, so a channel just added or removed moves between
	// the list and the picker.
	const available = resource(
		() => ({ ws: adding && canPick ? workspace : undefined, channels }),
		async ({ ws, channels }) => (ws ? await listChoices(ws, channels) : [])
	)

	async function listChoices(ws: string, shared: SlackAgentChannel[]): Promise<Choice[]> {
		try {
			const all: Choice[] = []
			let cursor: string | undefined = undefined
			// ponytail: stops at 10 pages of 200 channels; search on the server if a team has more.
			for (let page = 0; page < 10; page++) {
				const res = await SlackAgentsService.listAvailableSlackChannels({ workspace: ws, cursor })
				for (const c of res.channels) {
					if (shared.some((s) => s.channel_id === c.id)) continue
					all.push({ value: c.id, name: c.name, label: `#${c.name}` })
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

	async function share(channelId: string, channelName: string, isDefault: boolean) {
		if (!workspace) return
		saving = true
		try {
			await SlackAgentsService.shareAgentInSlackChannel({
				workspace,
				path,
				requestBody: { channel_id: channelId, channel_name: channelName, is_default: isDefault }
			})
			adding = false
		} catch (e) {
			sendUserToast(e.body ?? e.message, true)
		} finally {
			saving = false
			// Also on failure, so a default toggle flipped in vain flips back.
			slack.refetch()
		}
	}

	async function unshare(channel: SlackAgentChannel) {
		if (!workspace) return
		try {
			await SlackAgentsService.unshareAgentFromSlackChannel({
				workspace,
				path,
				channelId: channel.channel_id
			})
			slack.refetch()
		} catch (e) {
			sendUserToast(e.body ?? e.message, true)
		}
	}
</script>

{#if draft.state}
	<div class="flex flex-col gap-4">
		<Label label="Who can ask it in Slack">
			<div class="flex flex-col gap-1">
				<ToggleButtonGroup
					bind:selected={
						() =>
							draft.state?.args.run_on_behalf_of && draft.state.args.slack_anyone
								? 'anyone'
								: 'members',
						(v) => {
							if (draft.state) draft.state.args.slack_anyone = v === 'anyone' || undefined
						}
					}
					disabled={readOnly}
					noWFull
				>
					{#snippet children({ item })}
						<ToggleButton value="members" label="Workspace members" {item} small />
						<ToggleButton
							value="anyone"
							label="Anyone in its channels"
							{item}
							small
							disabled={!draft.state?.args.run_on_behalf_of}
							tooltip={draft.state?.args.run_on_behalf_of
								? undefined
								: 'Turn on Run on behalf of first: everyone in the channels runs it as that user.'}
						/>
					{/snippet}
				</ToggleButtonGroup>
				<span class="text-2xs text-hint">
					{#if draft.state.args.run_on_behalf_of && draft.state.args.slack_anyone}
						Anyone in its channels, including people without a Windmill account, runs it as the user
						it runs on behalf of. Applies when you deploy.
					{:else}
						Members who can read the agent, matched by their Slack email, each running it as they
						would here.
					{/if}
				</span>
			</div>
		</Label>

		<Label label="Slack channels">
			<div class="flex flex-col gap-2">
				{#if draft.noDeployed}
					<span class="text-2xs text-hint">Deploy the agent to share it in Slack channels.</span>
				{:else if slack.loading && !slack.current}
					<span class="text-2xs text-hint">Loading…</span>
				{:else if !slack.current?.slack_team_name}
					<span class="text-2xs text-hint">
						Slack isn't connected to the workspace.
						{#if isAdmin}
							<a href="/workspace_settings?tab=slack&agents=true">Connect it</a> with AI agents enabled.
						{/if}
					</span>
				{:else}
					{#if channels.length === 0 && !adding}
						<span class="text-2xs text-hint">Not shared in any channel.</span>
					{/if}
					{#each channels as channel (channel.channel_id)}
						<div class="flex items-center gap-2 w-full">
							<TextInput
								class="flex-1"
								inputProps={{ readonly: true }}
								value="#{channel.channel_name}"
							/>
							<Toggle
								size="xs"
								checked={channel.is_default}
								disabled={readOnly || (!isAdmin && !channel.is_default) || saving}
								on:change={(e) => share(channel.channel_id, channel.channel_name, e.detail)}
								options={{
									right: 'Default',
									rightTooltip:
										"Answers the bot's mentions and /windmill commands in this channel that name no agent, which then no longer reach the workspace's command script there. Only workspace admins can turn it on."
								}}
							/>
							{#if !readOnly}
								<Button
									variant="subtle"
									unifiedSize="md"
									iconOnly
									destructive
									startIcon={{ icon: X }}
									title="Stop sharing in #{channel.channel_name}"
									onClick={() => unshare(channel)}
								/>
							{/if}
						</div>
					{/each}
					{#if canPick}
						{#if adding}
							<div class="flex items-center gap-2 w-full">
								<Select
									class="flex-1"
									items={available.current ?? []}
									bind:value={
										() => undefined,
										(v) => {
											const choice = available.current?.find((c) => c.value === v)
											if (choice) share(choice.value, choice.name, false)
										}
									}
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
									Share in a channel
								</Button>
							</div>
						{/if}
						<span class="text-2xs text-hint">Channel changes apply at once.</span>
					{:else if !readOnly}
						<span class="text-2xs text-hint">
							Slack can't be reached or was connected without AI agents; the agent's page says why.
						</span>
					{/if}
				{/if}
			</div>
		</Label>
	</div>
{/if}
