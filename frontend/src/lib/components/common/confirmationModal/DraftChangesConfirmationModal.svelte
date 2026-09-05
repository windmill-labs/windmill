<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import type { Trigger } from '$lib/components/triggers/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { twMerge } from 'tailwind-merge'
	import TriggerLabel from '$lib/components/triggers/TriggerLabel.svelte'
	import { triggerIconMap } from '$lib/components/triggers/utils'
	import { Bot, Star } from 'lucide-svelte'
	import ToggleButtonGroup from '../toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../toggleButton-v2/ToggleButton.svelte'
	import { userStore } from '$lib/stores'
	import Badge from '../badge/Badge.svelte'
	import type { LinkedAgentDraft } from '$lib/components/flows/linkedAgentDrafts'

	interface Props {
		open?: boolean
		draftTriggers?: Trigger[]
		/** Saved agents this flow links to that have an unsaved draft. Scripts pass none: only a
		 *  flow step can link an agent. */
		draftAgents?: LinkedAgentDraft[]
		/** Whether this user may write each listed agent's resource, keyed by path. */
		agentCanWrite?: Record<string, boolean>
		/** Why an agent cannot be deployed, keyed by path, from the same rule the agent editor's own
		 *  Deploy button follows — so this dialog cannot offer a write that would be rejected. Decided
		 *  by the caller: this is a generic dialog the script editor mounts too, and agent validation
		 *  has no business in its bundle. */
		agentRefusal?: Record<string, string | undefined>
		isFlow?: boolean
	}

	let {
		open = $bindable(false),
		draftTriggers = [],
		draftAgents = [],
		agentCanWrite = {},
		agentRefusal = {},
		isFlow = false
	}: Props = $props()

	let selectedTriggers: Trigger[] = $state(untrack(() => draftTriggers))
	let selectedAgents: LinkedAgentDraft[] = $state([])

	const dispatch = createEventDispatcher<{
		canceled: void
		confirmed: { selectedTriggers: Trigger[]; selectedAgents: LinkedAgentDraft[] }
	}>()

	function toggleTrigger(trigger: Trigger, selected: 'discard' | 'deploy') {
		if (selected === 'discard') {
			if (trigger.isDraft) {
				selectedTriggers = selectedTriggers.filter((t) => !t.isDraft || t.id !== trigger.id)
			} else {
				selectedTriggers = selectedTriggers.filter(
					(t) => t.isDraft || t.type !== trigger.type || t.path !== trigger.path
				)
			}
		} else if (!isSelected(selectedTriggers, trigger)) {
			selectedTriggers = [...selectedTriggers, trigger]
		}
	}

	function isSelected(triggers: Trigger[], trigger: Trigger): boolean {
		if (trigger.isDraft) {
			return triggers.some((t) => t.id === trigger.id)
		} else {
			return triggers.some((t) => t.path === trigger.path && t.type === trigger.type)
		}
	}

	function toggleAgent(agent: LinkedAgentDraft, selected: 'discard' | 'deploy') {
		if (selected === 'discard') {
			selectedAgents = selectedAgents.filter((a) => a.path !== agent.path)
		} else if (!selectedAgents.some((a) => a.path === agent.path)) {
			selectedAgents = [...selectedAgents, agent]
		}
	}

	function checkSavePermissions(trigger: Trigger) {
		// Creating http trigger is forbidden for non-admin users
		const adminOnly =
			trigger.type === 'http' &&
			!$userStore?.is_admin &&
			!$userStore?.is_super_admin &&
			trigger.isDraft

		const invalidConfig = !trigger.draftConfig?.canSave

		return invalidConfig ? 'invalid-config' : adminOnly ? 'admin-only' : 'deploy'
	}

	function checkAgentPermissions(agent: LinkedAgentDraft): {
		state: 'deploy' | 'read-only' | 'invalid-config'
		reason?: string
	} {
		if (agentCanWrite[agent.path] === false) {
			return { state: 'read-only' }
		}
		const refusal = agentRefusal[agent.path]
		return refusal ? { state: 'invalid-config', reason: refusal } : { state: 'deploy' }
	}

	$effect(() => {
		if (!open) return
		selectedTriggers = [...draftTriggers].filter((t) => checkSavePermissions(t) === 'deploy')
		selectedAgents = [...draftAgents].filter((a) => checkAgentPermissions(a).state === 'deploy')
	})

	const runnable = $derived(isFlow ? 'flow' : 'script')
</script>

<ConfirmationModal
	{open}
	title="Unsaved changes detected"
	confirmationText={isFlow ? 'Deploy Flow' : 'Deploy Script'}
	type="reload"
	showIcon={false}
	on:canceled={() => dispatch('canceled')}
	on:confirmed={() => dispatch('confirmed', { selectedTriggers, selectedAgents })}
>
	<div class="flex flex-col w-full gap-8 pb-4">
		{#if draftTriggers.length > 0}
			<div class="flex flex-col gap-2">
				<div class="text-secondary text-sm">
					{`Your ${runnable} has draft triggers. Select which draft triggers to deploy with the ${runnable}. Undeployed draft triggers will be permanently deleted.`}
				</div>

				<div class={draftTriggers.length > 5 ? 'h-[300px]' : ''}>
					<DataTable size="sm" tableFixed={true}>
						<thead>
							<tr class="bg-gray-50 dark:bg-gray-700 text-secondary dark:text-gray-300 text-xs">
								<th class="text-left py-2 px-4">Triggers to deploy</th>
								<th class="w-32 text-center py-2 px-1 justify-center"> </th>
							</tr>
						</thead>
						<tbody>
							{#each draftTriggers as trigger}
								{@const SvelteComponent = triggerIconMap[trigger.type]}
								{@const permission = checkSavePermissions(trigger)}
								{@const isSelectedTrigger = isSelected(selectedTriggers, trigger)}
								<tr
									class={twMerge(
										'transition-colors h-12 border-t border-gray-200 dark:border-gray-700 whitespace-nowrap',
										permission === 'deploy' ? 'hover:bg-surface-hover ' : ''
									)}
								>
									<td class={twMerge('text-center py-1 px-4')}>
										<div class="flex flex-row items-center gap-2">
											<div class="relative flex justify-center items-center">
												<SvelteComponent
													size={14}
													class={isSelectedTrigger ? 'text-accent' : 'text-hint'}
												/>
												{#if trigger.isPrimary}
													<Star size={8} class="absolute -mt-3 ml-3 text-accent" />
												{/if}
											</div>
											<div class="flex grow min-w-0 items-center text-left">
												<TriggerLabel {trigger} discard={!isSelectedTrigger} />
											</div>
										</div>
									</td>

									<td class="text-left py-1">
										{#if permission === 'deploy'}
											<div class="flex justify-start">
												<ToggleButtonGroup
													class="w-fit h-fit"
													selected={isSelectedTrigger ? 'deploy' : 'discard'}
													on:selected={(e) => toggleTrigger(trigger, e.detail)}
												>
													{#snippet children({ item })}
														<ToggleButton
															label={!trigger.isDraft && trigger.draftConfig ? 'Reset' : 'Discard'}
															value={'discard'}
															{item}
															small
															class="data-[state=on]:text-white data-[state=on]:bg-red-400 justify-center"
														/>
														<ToggleButton
															label={!trigger.isDraft && trigger.draftConfig ? 'Update' : 'Deploy'}
															value={'deploy'}
															{item}
															small
															class="data-[state=on]:bg-surface-accent-primary data-[state=on]:text-white justify-center"
														/>
													{/snippet}
												</ToggleButtonGroup>
											</div>
										{:else if permission === 'admin-only'}
											<Badge color="red">Admin only</Badge>
										{:else if permission === 'invalid-config'}
											<Badge color="red">Invalid config</Badge>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</DataTable>
				</div>
			</div>
		{/if}

		{#if draftAgents.length > 0}
			<div class="flex flex-col gap-2">
				<div class="text-secondary text-sm">
					Saved agents this flow uses have unsaved changes. Select which ones to deploy with the
					flow. An agent kept as a draft stays editable, and the flow runs the agent as currently
					deployed.
				</div>

				<div class={draftAgents.length > 5 ? 'h-[300px]' : ''}>
					<DataTable size="sm" tableFixed={true}>
						<thead>
							<tr class="bg-gray-50 dark:bg-gray-700 text-secondary dark:text-gray-300 text-xs">
								<th class="text-left py-2 px-4">Agents to deploy</th>
								<!-- Wider than the trigger table's: "Keep as draft" does not fit its w-32. -->
								<th class="w-48 text-center py-2 px-1 justify-center"> </th>
							</tr>
						</thead>
						<tbody>
							{#each draftAgents as agent (agent.path)}
								{@const permission = checkAgentPermissions(agent)}
								{@const isSelectedAgent = selectedAgents.some((a) => a.path === agent.path)}
								<!-- `min-h`, not the trigger table's fixed `h-12`: a never-deployed agent's row carries
								     a second line of warning under the path. -->
								<tr
									class={twMerge(
										'transition-colors border-t border-gray-200 dark:border-gray-700',
										permission.state === 'deploy' ? 'hover:bg-surface-hover ' : ''
									)}
								>
									<td class="min-h-12 text-center py-2 px-4">
										<div class="flex flex-row items-center gap-2">
											<Bot size={14} class={isSelectedAgent ? 'text-accent' : 'text-hint'} />
											<div class="flex grow min-w-0 flex-col text-left">
												<div class="flex items-center gap-2 min-w-0">
													<!-- Dimmed rather than struck through: keeping a draft destroys nothing, so the
													     trigger table's deletion styling would say the wrong thing. -->
													<span
														class={twMerge(
															'truncate text-xs',
															isSelectedAgent ? '' : 'text-tertiary'
														)}
														title={agent.path}
													>
														{agent.path}
													</span>
													{#if agent.noDeployed}
														<Badge small color="indigo">Never deployed</Badge>
													{/if}
												</div>
												{#if agent.noDeployed && !isSelectedAgent}
													<!-- The section's promise — the flow falls back to the deployed agent — does not
													     hold for one that has never been deployed. There is nothing to fall back to,
													     so the flow would land naming a path that does not resolve. -->
													<span class="text-xs text-red-600 dark:text-red-400 whitespace-normal">
														Never deployed, so the flow will not run until this agent is deployed.
													</span>
												{/if}
											</div>
										</div>
									</td>

									<td class="text-left py-1">
										{#if permission.state === 'deploy'}
											<div class="flex justify-start">
												<ToggleButtonGroup
													class="w-fit h-fit"
													selected={isSelectedAgent ? 'deploy' : 'discard'}
													on:selected={(e) => toggleAgent(agent, e.detail)}
												>
													{#snippet children({ item })}
														<ToggleButton
															label="Keep as draft"
															value={'discard'}
															{item}
															small
															class="justify-center"
														/>
														<ToggleButton
															label={agent.noDeployed ? 'Deploy' : 'Update'}
															value={'deploy'}
															{item}
															small
															class="data-[state=on]:bg-surface-accent-primary data-[state=on]:text-white justify-center"
														/>
													{/snippet}
												</ToggleButtonGroup>
											</div>
										{:else if permission.state === 'read-only'}
											<Badge color="red" title="You do not have write access to this agent">
												Read-only
											</Badge>
										{:else}
											<Badge color="red" title={permission.reason}>Invalid config</Badge>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</DataTable>
				</div>
			</div>
		{/if}
	</div>
</ConfirmationModal>
