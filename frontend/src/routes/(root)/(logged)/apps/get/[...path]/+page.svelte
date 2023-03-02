<script lang="ts">
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import { Skeleton } from '$lib/components/common'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { classNames } from '$lib/utils'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	let app: AppWithLastVersion | undefined = undefined

	async function loadApp() {
		app = await AppService.getAppByPath({ workspace: $workspaceStore!, path: $page.params.path })
	}

	let queryId = $page.url.searchParams.get('workspace_id')
	if (queryId && queryId != $workspaceStore) {
		$workspaceStore = $page.url.searchParams.get('workspace_id')!
	}

	$: if ($workspaceStore) {
		loadApp()
	}

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

{#if app}
	<div
		class={twMerge('min-h-screen h-full w-full', app?.value.css?.['app']?.['viewer']?.class)}
		style={app?.value.css?.['app']?.['viewer']?.style}
	>
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
