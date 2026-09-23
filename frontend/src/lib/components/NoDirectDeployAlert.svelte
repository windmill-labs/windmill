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
	import { Button, ButtonType } from './common'
	import { twMerge } from 'tailwind-merge'
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

	// The Popover renders its trigger as a <button>, so this wears the design system's subtle
	// button rather than containing one — a <Button> here would nest a button inside a button.
	const triggerClass = twMerge(
		'inline-flex items-center gap-1 rounded-md whitespace-nowrap transition-all',
		ButtonType.VariantStyles.subtle,
		ButtonType.UnifiedSizingClasses.sm,
		ButtonType.UnifiedHeightClasses.sm,
		ButtonType.UnifiedFontSizes.sm,
		'text-xs'
	)
</script>

{#if !$userStore?.operator && activeDeployRulesets.length > 0}
	<div class="my-2">
		<Popover
			placement="bottom-start"
			class={triggerClass}
			triggerAttrs={{ 'aria-label': badgeLabel }}
		>
			{#snippet trigger()}
				{#if bypassActive}
					<ShieldOff size={ButtonType.UnifiedIconSizes.sm} class="shrink-0" />
				{:else if canonicalDev}
					<GitFork size={ButtonType.UnifiedIconSizes.sm} class="shrink-0" />
				{:else}
					<Lock size={ButtonType.UnifiedIconSizes.sm} class="shrink-0" />
				{/if}
				{badgeLabel}
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
