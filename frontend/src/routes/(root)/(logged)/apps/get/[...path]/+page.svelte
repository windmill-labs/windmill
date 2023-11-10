<script lang="ts">
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import { Button, Skeleton } from '$lib/components/common'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { Pen } from 'lucide-svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	let app: AppWithLastVersion | undefined = undefined
	let can_write = false

	async function loadApp() {
		app = await AppService.getAppByPath({ workspace: $workspaceStore!, path: $page.params.path })
		can_write = canWrite(app?.path, app?.extra_perms!, $userStore)
	}

	$: if ($workspaceStore && $page.params.path) {
		loadApp()
	}

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

{#if app}
	{#key app}
		<div
			class={twMerge(
				'min-h-screen h-full w-full',
				app?.value.css?.['app']?.['viewer']?.class,
				'wm-app-viewer'
			)}
			style={app?.value.css?.['app']?.['viewer']?.style}
		>
			<AppPreview
				context={{
					email: $userStore?.email,
					username: $userStore?.username,
					groups: $userStore?.groups,
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
				<div class="absolute bottom-4 z-50 right-4">
					<Button size="sm" startIcon={{ icon: Pen }} variant="border" href="/apps/edit/{app.path}"
						>Edit</Button
					>
				</div>
			{/if}
		</div>
	{/key}
{:else}
	<Skeleton layout={[10]} />
{/if}
