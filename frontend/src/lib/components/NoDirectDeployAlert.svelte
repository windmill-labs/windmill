<script lang="ts">
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import {
		canUserBypassRuleKind,
		getActiveRulesetsForKind,
		isRuleActive
	} from '$lib/workspaceProtectionRules.svelte'
	import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
	import { switchWorkspace } from '$lib/storeUtils'
	import { Alert, Button } from './common'
	import { GitFork } from 'lucide-svelte'

	let activeDeployRulesets = $derived(getActiveRulesetsForKind('DisableDirectDeployment'))
	let canBypass = $derived(canUserBypassRuleKind('DisableDirectDeployment', $userStore))
	let canonicalDev = $derived(findCanonicalDevWorkspace($workspaceStore, $userWorkspaces))
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
			title={canonicalDev ? 'Edits happen in the dev workspace' : 'Workspace protection active'}
		>
			<div class="flex flex-col gap-2">
				{#if canonicalDev}
					<p>
						This is a prod workspace. Make your changes in its dev workspace
						<b>{canonicalDev.name}</b> ({canonicalDev.id}) and promote them here.
					</p>
					<div>
						<Button
							size="xs"
							variant="default"
							startIcon={{ icon: GitFork }}
							onclick={() => {
								if (canonicalDev) switchWorkspace(canonicalDev.id)
							}}
						>
							Go to dev workspace
						</Button>
					</div>
				{:else}
					<p>
						The rule{activeDeployRulesets.length > 1 ? 's' : ''}
						<b>{activeDeployRulesets.map((r) => r.name).join(', ')}</b>
						restrict{activeDeployRulesets.length > 1 ? '' : 's'} direct edits to this workspace. You
						will need to either fork the workspace, or make your changes locally and submit a PR to an
						authorized user.
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
