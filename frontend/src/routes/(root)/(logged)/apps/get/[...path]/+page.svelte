<script lang="ts">
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import { Button, Skeleton } from '$lib/components/common'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { faPen } from '@fortawesome/free-solid-svg-icons'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	let app: AppWithLastVersion | undefined = undefined
	let can_write = false

	async function loadApp() {
		app = await AppService.getAppByPath({ workspace: $workspaceStore!, path: $page.params.path })
		can_write = canWrite(app?.path, app?.extra_perms!, $userStore)
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
				query: Object.fromEntries($page.url.searchParams.entries()),
				hash: $page.url.hash
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
		{#if can_write}
			<div class="absolute bottom-4 z-20 right-4">
				<Button size="sm" startIcon={{ icon: faPen }} variant="border" href="/apps/edit/{app.path}"
					>Edit</Button
				>
			</div>
		{/if}
	</div>
{:else}
	<Skeleton layout={[10]} />
{/if}
