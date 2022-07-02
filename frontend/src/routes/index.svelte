<script lang="ts">
	import { goto } from '$app/navigation'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { workspaceStore } from '$lib/stores'

	$workspaceStore = localStorage.getItem('workspace') ?? undefined

	if ($workspaceStore) {
		goto('/scripts')
	}
</script>

<CenteredPage>
	<div class="py-6">
		<h1>Welcome to windmill.dev !</h1>
	</div>
</CenteredPage>
