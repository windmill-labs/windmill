<script lang="ts">
	/**
	 * Badge for a workspace locked against direct edits, with the rule names, the route into the dev
	 * workspace and the admin bypass in its popover. Renders nothing for operators, who have no edit
	 * affordance for it to explain.
	 */
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import {
		canUserBypassRuleKind,
		getActiveRulesetsForKind,
		isRuleActive
	} from '$lib/workspaceProtectionRules.svelte'
	import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
	import { devLabelKey, devLabelNoun } from '$lib/utils/devWorkspaceLabel'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { switchWorkspace } from '$lib/storeUtils'
	import { Badge, Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import Toggle from './Toggle.svelte'
	import { GitFork, Lock, ShieldOff } from 'lucide-svelte'

	let activeDeployRulesets = $derived(getActiveRulesetsForKind('DisableDirectDeployment'))
	let canBypass = $derived(canUserBypassRuleKind('DisableDirectDeployment', $userStore))
	let canonicalDev = $derived(findCanonicalDevWorkspace($workspaceStore, $userWorkspaces))
	// Forking may itself be blocked by DisableWorkspaceForking, so only suggest it
	// when the user can actually fork this workspace.
	let canFork = $derived(canCreateFork($userStore))
	let editAdvice = $derived(
		canFork
			? 'You will need to either fork the workspace, or make your changes locally and submit a PR to an authorized user.'
			: 'You will need to make your changes locally and submit a PR to an authorized user.'
	)
	let overrideChecked = $state(false)
	// The toggle is only offered to a user who can bypass, but the answer can change under a
	// workspace switch, so the checked flag alone never grants the edit.
	let bypassActive = $derived(canBypass && overrideChecked)
	let canEdit = $derived(!isRuleActive('DisableDirectDeployment') || bypassActive)

	let {
		onUpdateCanEditStatus = (value) => {}
	}: {
		onUpdateCanEditStatus?: (value: boolean) => void
	} = $props()

	$effect(() => {
		onUpdateCanEditStatus(canEdit)
	})

	let badgeLabel = $derived(
		bypassActive
			? 'Protection bypassed'
			: canonicalDev
				? `Edits in ${devLabelKey(canonicalDev.dev_workspace_label)}`
				: 'Edits restricted'
	)
</script>

{#if !$userStore?.operator && activeDeployRulesets.length > 0}
	<div class="my-2">
		<Popover
			placement="bottom-start"
			class="inline-flex items-center"
			triggerAttrs={{ 'aria-label': badgeLabel }}
		>
			{#snippet trigger()}
				<!-- `clickable` is unusable here: it renders the badge as a <button>, nested inside the
				     one Popover wraps its trigger in. -->
				<Badge small color={bypassActive ? 'yellow' : 'blue'} class="cursor-pointer">
					{#if bypassActive}
						<ShieldOff class="h-3 w-3" />
					{:else if canonicalDev}
						<GitFork class="h-3 w-3" />
					{:else}
						<Lock class="h-3 w-3" />
					{/if}
					{badgeLabel}
				</Badge>
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col gap-3 p-4 text-xs max-w-sm">
					{#if canonicalDev}
						<p class="text-primary">
							Edits to this workspace are made in its {devLabelNoun(
								canonicalDev.dev_workspace_label
							)}
							<b>{canonicalDev.name}</b> ({canonicalDev.id}) and promoted here.
						</p>
						<div>
							<Button
								btnClasses="w-auto"
								size="xs"
								variant="accent"
								startIcon={{ icon: GitFork }}
								onclick={() => {
									if (canonicalDev) switchWorkspace(canonicalDev.id)
								}}
							>
								Go to {devLabelNoun(canonicalDev.dev_workspace_label)}
							</Button>
						</div>
					{:else}
						<p class="text-primary">
							The rule{activeDeployRulesets.length > 1 ? 's' : ''}
							<b>{activeDeployRulesets.map((r) => r.name).join(', ')}</b>
							restrict{activeDeployRulesets.length > 1 ? '' : 's'} direct edits to this workspace.
							{editAdvice}
						</p>
					{/if}
					{#if canBypass}
						<Toggle
							size="xs"
							bind:checked={overrideChecked}
							options={{ right: 'Bypass restriction' }}
						/>
					{/if}
				</div>
			{/snippet}
		</Popover>
	</div>
{/if}
