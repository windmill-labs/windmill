<script lang="ts">
	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { page } from '$app/stores'

	let app: AppWithLastVersion | undefined = undefined
	let path = $page.params.path

	async function loadApp(): Promise<void> {
		app = await AppService.getAppByPath({
			path,
			workspace: $workspaceStore!
		})
	}

	$: {
		if ($workspaceStore) {
			loadApp()
		}
	}
</script>

{#if app}
	<div class="h-screen">
		<AppEditor app={app.value} path={app.path} />
	</div>
{/if}
