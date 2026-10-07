<script lang="ts">
	import CreateWorkspace from '$lib/components/workspaceSettings/CreateWorkspace.svelte'
	import { userStore } from '$lib/stores'
</script>

<!-- The form is reachable by URL whatever the pickers show, and forking is what turns an operator
     into a full member of the copy (the fork's membership row carries `is_admin` but not
     `operator`). The server still accepts the call — this refuses the page, it does not enforce
     the rule; see docs/operator-write-rights.md. -->
{#if $userStore?.operator}
	<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4 mt-12" role="alert">
		<p class="font-bold">Unauthorized</p>
		<p>Page not available for operators</p>
	</div>
{:else}
	<CreateWorkspace isFork={true} />
{/if}
