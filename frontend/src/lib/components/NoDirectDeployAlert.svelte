<script lang="ts">
	import { userStore } from '$lib/stores'
	import {
		canUserBypassRuleKind,
		getActiveRulesetsForKind,
		isRuleActive
	} from '$lib/workspaceProtectionRules.svelte'
	import { Alert } from './common'

	let activeDeployRulesets = $derived(getActiveRulesetsForKind('RequireForkOrBranchToDeploy'))
	let canBypass = $derived(canUserBypassRuleKind('RequireForkOrBranchToDeploy', $userStore))
	let overrideChecked = $state(false)
	let canEdit = $derived(
		!isRuleActive('RequireForkOrBranchToDeploy') || (canBypass && overrideChecked)
	)

	let {
		onUpdateCanEditStatus = (value) => {}
	}: {
		onUpdateCanEditStatus?: (value: boolean) => void
	} = $props()

	$effect(() => {
		onUpdateCanEditStatus(canEdit)
	})
</script>

{#if !$userStore?.operator && activeDeployRulesets.length > 0}
	<div class="my-2">
		<Alert type="info" title="Workspace protection active">
			<div class="flex flex-col gap-2">
				<p>
					The rule{activeDeployRulesets.length > 1 ? "s" : ""} <b>{activeDeployRulesets.map((r) => r.name).join(', ')}</b> restrict{activeDeployRulesets.length > 1 ? "" : "s"} direct edits to
					this workspace. Use either a fork and the deployment UI, or a git sync based workflow (such as a PR on your synced repo) to make changes.
				</p>
				{#if canBypass}
					<label class="flex items-center gap-2 cursor-pointer">
						<input class="rounded max-w-4" type="checkbox" bind:checked={overrideChecked} />
						<span class="text-xs">Bypass restriction</span>
					</label>
				{/if}
			</div>
		</Alert>
	</div>
{/if}
