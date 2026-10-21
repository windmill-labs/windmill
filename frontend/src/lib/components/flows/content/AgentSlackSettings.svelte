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
		/** Who the next deploy makes the agent run as, when it runs on behalf of someone. */
		nextRunAs: string | undefined
	}

	let { draft, path, workspace, nextRunAs }: Props = $props()

	let readOnly = $derived(!draft.canWrite)
	let obo = $derived(!!draft.state?.args.run_on_behalf_of)
	let runsAs = $derived(
		readOnly
			? (draft.onBehalfOf ?? 'its run-as user')
			: !nextRunAs || nextRunAs === `u/${$userStore?.username}`
				? 'you'
				: nextRunAs
	)
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
		<Label label="Who can ask in its channels">
			<ToggleButtonGroup
				bind:selected={
					() => (obo && draft.state?.args.slack_anyone ? 'anyone' : 'members'),
					(v) => {
						if (!draft.state) return
						draft.state.args.slack_anyone = v === 'anyone' || undefined
						// Anyone there runs it as its identity, so there must be one.
						if (v === 'anyone') draft.state.args.run_on_behalf_of = true
					}
				}
				disabled={readOnly}
				noWFull
			>
				{#snippet children({ item })}
					<ToggleButton
						value="members"
						label={obo ? `Workspace members, as ${runsAs}` : 'Workspace members, as themselves'}
						tooltip={`Members who can read the agent, matched by their Slack email. ${obo ? `They run it as ${runsAs}, as they would in Windmill.` : 'Each runs it with their own permissions, as they would in Windmill.'}`}
						showTooltipIcon
						{item}
						small
					/>
					<ToggleButton
						value="anyone"
						label="Anyone, as {runsAs}"
						tooltip={`Everyone in these channels, people without a Windmill account included, runs it as ${runsAs}. ${obo ? '' : 'Turns on Run on behalf of, which also applies to runs from Windmill and the API. '}Applies when you deploy.`}
						showTooltipIcon
						{item}
						small
					/>
				{/snippet}
			</ToggleButtonGroup>
		</Label>
		<div class="flex flex-col gap-2 rounded-md border p-3">
			<span class="text-xs font-semibold text-emphasis">Channels</span>
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
										'Answers bot mentions in this channel that name no agent. Admins only.'
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
						<span class="text-2xs text-hint"
							>Channels are not saved with the agent: changes apply directly.</span
						>
					{:else if !readOnly}
						<span class="text-2xs text-hint">
							Slack can't be reached or was connected without AI agents; the agent's page says why.
						</span>
					{/if}
				{/if}
			</div>
		</div>
	</div>
{/if}
