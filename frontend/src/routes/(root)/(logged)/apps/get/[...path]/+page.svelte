<script lang="ts">
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import { Skeleton } from '$lib/components/common'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { writable } from 'svelte/store'

	let app: AppWithLastVersion | undefined = undefined

	async function loadApp() {
		app = await AppService.getAppByPath({ workspace: $workspaceStore!, path: $page.params.path })
	}

	$: if ($workspaceStore) {
		loadApp()
	}

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

{#if app}
	<div class="border rounded-md p-2 w-full">
		<AppPreview
			context={{
				email: $userStore?.email,
				username: $userStore?.username,
				query: Object.fromEntries($page.url.searchParams.entries())
			}}
			workspace={$workspaceStore ?? ''}
			summary={app.summary}
			app={app.value}
			appPath={app.path}
			{breakpoint}
			policy={app.policy}
			isEditor={false}
			noBackend={false}
		/>
	</div>
{:else}
	<Skeleton layout={[10]} />
{/if}
