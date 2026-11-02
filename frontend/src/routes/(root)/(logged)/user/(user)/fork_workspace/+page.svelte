<script lang="ts">
	import CreateWorkspace from '$lib/components/workspaceSettings/CreateWorkspace.svelte'
	import { Alert } from '$lib/components/common'
	import { roleCapabilities } from '$lib/stores'
	import { canFork } from '$lib/roleCapabilities'
</script>

<!-- The form is reachable by URL whatever the pickers show; the server refuses the fork too. -->
{#if !canFork($roleCapabilities)}
	<div class="p-4 mt-12">
		<Alert type="error" title="Unauthorized">Page not available for your role</Alert>
	</div>
{:else}
	<CreateWorkspace isFork={true} />
{/if}
