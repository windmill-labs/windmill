<script lang="ts">
	import type { ProtectionRuleset } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import {
		fetchProtectionRulesForWorkspace,
		isRuleActiveInRulesets,
		canUserBypassRuleKindInRulesets,
		getActiveRulesetsForKindInRulesets,
	} from '$lib/workspaceProtectionRules.svelte'
	import { Alert } from './common'
	import { untrack } from 'svelte'

	let {
		parentWorkspaceId,
		onUpdateCanDeploy = (value) => {}
	}: {
		parentWorkspaceId: string
		onUpdateCanDeploy?: (value: boolean) => void
	} = $props()

	let parentRulesets = $state<ProtectionRuleset[]>([])
	let overrideChecked = $state(false)

	let activeDeployRulesets = $derived(
		getActiveRulesetsForKindInRulesets(parentRulesets, 'DisableDirectDeployment')
	)

	let canBypass = $derived(
		canUserBypassRuleKindInRulesets(parentRulesets, 'DisableDirectDeployment', $userStore)
	)

	let canDeploy = $derived(
		!isRuleActiveInRulesets(parentRulesets, 'DisableDirectDeployment') ||
			(canBypass && overrideChecked)
	)

	// Fetch parent workspace rules
	$effect(() => {
		if (parentWorkspaceId) {
			untrack(async () => {
				const rules = await fetchProtectionRulesForWorkspace(parentWorkspaceId)
				parentRulesets = rules
			})
		}
	})

	// Reset override when parent workspace changes
	$effect(() => {
		parentWorkspaceId
		overrideChecked = false
	})

	// Communicate deployment status to parent
	$effect(() => {
		onUpdateCanDeploy(canDeploy)
	})
</script>

{#if !$userStore?.operator && activeDeployRulesets.length > 0}
	<Alert type="info" title="Parent workspace protection active" class="my-2">
		<div class="flex flex-col gap-2">
			<p>
				The workspace {parentWorkspaceId} has a protection rule{activeDeployRulesets.length > 1 ? 's' : ''}
				<b>{activeDeployRulesets.map((r) => r.name).join(', ')}</b>
				that restrict{activeDeployRulesets.length > 1 ? '' : 's'} direct deployments. You need to merge changes through the synced git repo with Git Sync, or by asking a user with the rights to bypass this rule.
			</p>
			{#if canBypass}
				<label class="flex items-center gap-2 cursor-pointer">
					<input class="rounded max-w-4" type="checkbox" bind:checked={overrideChecked} />
					<span class="text-xs">Bypass restriction and deploy anyway</span>
				</label>
			{/if}
		</div>
	</Alert>
{/if}
