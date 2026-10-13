<script lang="ts">
	import CreateWorkspace from '$lib/components/workspaceSettings/CreateWorkspace.svelte'
	import { Alert } from '$lib/components/common'
	import { userStore } from '$lib/stores'
</script>

<!-- The form is reachable by URL whatever the pickers show, and forking is what turns an operator
     into a full member of the copy (the fork's membership row carries `is_admin` but not
     `operator`). The server still accepts the call — this refuses the page, it does not enforce
     the rule; see docs/operator-write-rights.md. -->
{#if $userStore?.operator}
	<div class="p-4 mt-12">
		<Alert type="error" title="Unauthorized">Page not available for operators</Alert>
	</div>
{:else}
	<CreateWorkspace isFork={true} />
{/if}
