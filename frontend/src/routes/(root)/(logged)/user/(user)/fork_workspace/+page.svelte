<script lang="ts">
	import CreateWorkspace from '$lib/components/workspaceSettings/CreateWorkspace.svelte'
	import { Alert } from '$lib/components/common'
	import { roleCapabilities } from '$lib/stores'
</script>

<!-- The form is reachable by URL whatever the pickers show. The server refuses a fork to a role
     without `fork` either way; a developer blocked by `DisableWorkspaceForking` gets the server's
     ruleset message instead. -->
{#if !$roleCapabilities.fork}
	<div class="p-4 mt-12">
		<Alert type="error" title="Unauthorized">Page not available for your role</Alert>
	</div>
{:else}
	<CreateWorkspace isFork={true} />
{/if}
