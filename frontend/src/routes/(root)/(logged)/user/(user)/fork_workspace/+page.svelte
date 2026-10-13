<script lang="ts">
	import CreateWorkspace from '$lib/components/workspaceSettings/CreateWorkspace.svelte'
	import { Alert } from '$lib/components/common'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canCreateFork } from '$lib/utils/editInFork'
	import { loadProtectionRules, protectionRulesState } from '$lib/workspaceProtectionRules.svelte'

	// This route's layout resets to the root one, so the logged layout that loads the rulesets
	// never mounts on a direct visit, and `AllowOperatorForking` would read as off.
	if ($workspaceStore && protectionRulesState.workspace !== $workspaceStore) {
		loadProtectionRules($workspaceStore)
	}
</script>

<!-- The form is reachable by URL whatever the pickers show. Only operators are refused here: the
     server refuses their fork unless a ruleset carries `AllowOperatorForking`, and a developer
     blocked by `DisableWorkspaceForking` gets the server's ruleset message instead. -->
{#if $userStore?.operator && !canCreateFork($userStore)}
	<div class="p-4 mt-12">
		<Alert type="error" title="Unauthorized">Page not available for operators</Alert>
	</div>
{:else}
	<CreateWorkspace isFork={true} />
{/if}
