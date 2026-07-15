<script lang="ts">
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import {
		canUserBypassRuleKind,
		getActiveRulesetsForKind,
		isRuleActive
	} from '$lib/workspaceProtectionRules.svelte'
	import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
	import { devLabelNoun } from '$lib/utils/devWorkspaceLabel'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { switchWorkspace } from '$lib/storeUtils'
	import { Alert, Button } from './common'
	import { GitFork } from 'lucide-svelte'

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
	let canEdit = $derived(!isRuleActive('DisableDirectDeployment') || (canBypass && overrideChecked))

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
		<Alert
			type="info"
			title={canonicalDev
				? `Edits happen in the ${devLabelNoun(canonicalDev.dev_workspace_label)}`
				: 'Workspace protection active'}
		>
			<div class="flex flex-col gap-2">
				{#if canonicalDev}
					<p>
						Edits to this workspace are made in its {devLabelNoun(canonicalDev.dev_workspace_label)}
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
					<p>
						The rule{activeDeployRulesets.length > 1 ? 's' : ''}
						<b>{activeDeployRulesets.map((r) => r.name).join(', ')}</b>
						restrict{activeDeployRulesets.length > 1 ? '' : 's'} direct edits to this workspace.
						{editAdvice}
					</p>
				{/if}
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
