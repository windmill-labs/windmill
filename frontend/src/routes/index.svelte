<script lang="ts">
	import { goto } from '$app/navigation'
	import CenteredPage from '$lib/components/CenteredPage.svelte'

	goto('/scripts')
</script>

<CenteredPage>
	<div class="py-6">
		<h1>Welcome to windmill.dev !</h1>
	</div>
</CenteredPage>
