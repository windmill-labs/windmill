<script lang="ts">
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import { Skeleton } from '$lib/components/common'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { writable } from 'svelte/store'

	let app: AppWithLastVersion | undefined = undefined

	async function loadApp() {
		app = await AppService.getPublicAppBySecret({
			workspace: $page.params.workspace,
			path: $page.params.secret
		})
	}

	$: if ($workspaceStore) {
		loadApp()
	}

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

<!-- {JSON.stringify(app)} -->
{#if app}
	<div class="border rounded-md p-2 w-full">
		<AppPreview
			summary={app.summary}
			app={app.value}
			appPath={app.path}
			{breakpoint}
			policy={app.policy}
		/>
	</div>
{:else}
	<Skeleton layout={[10]} />
{/if}
