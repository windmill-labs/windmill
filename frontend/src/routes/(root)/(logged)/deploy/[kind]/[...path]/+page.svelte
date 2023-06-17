<script lang="ts">
	import { page } from '$app/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import DeployWorkspace from '$lib/components/DeployWorkspace.svelte'

	$: kind = asKind($page.params.kind)

	function asKind(
		kind: string
	): 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app' | 'raw_app' {
		return kind as 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app' | 'raw_app'
	}
</script>

<CenteredPage>
	<h1 class="mt-6">Deploy {$page.params.kind} {$page.params.path}</h1>
	<DeployWorkspace {kind} initialPath={$page.params.path} />
</CenteredPage>
