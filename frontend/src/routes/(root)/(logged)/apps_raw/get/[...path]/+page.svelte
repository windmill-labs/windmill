<script lang="ts">
	import { base } from '$app/paths'
	import { page } from '$app/stores'
	import { Button, Skeleton } from '$lib/components/common'
	import { AppService, type AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { Pen } from 'lucide-svelte'
	import RawAppPreview from '$lib/components/raw_apps/RawAppPreview.svelte'
	import type { HiddenRunnable } from '$lib/components/apps/types'

	const hideEditBtn = $page.url.searchParams.get('hideEditBtn') === 'true'

	let app: AppWithLastVersion | undefined = undefined

	async function loadApp() {
		console.log('Loading app')
		app = await AppService.getAppLiteByPath({
			workspace: $workspaceStore!,
			path: $page.params.path
		})
	}

	$: $workspaceStore && loadApp()

	$: can_write = canWrite($page.params.path, app?.extra_perms ?? {}, $userStore)
	function getRunnables(app: AppWithLastVersion) {
		return (app?.value?.runnables ?? {}) as Record<string, HiddenRunnable>
	}
	function getVersion(app: AppWithLastVersion) {
		return app?.value?.version as number
	}
</script>

<div class="h-full min-h-[600px] w-full relative p-2bg-white">
	{#if !$workspaceStore || !$userStore || !app}
		<Skeleton layout={[10]} />
	{:else}
		<RawAppPreview
			path={$page.params.path}
			workspace={$workspaceStore}
			user={$userStore}
			runnables={getRunnables(app)}
			version={getVersion(app)}
		/>
	{/if}
	{#if can_write && !hideEditBtn}
		<div id="app-edit-btn" class="absolute bottom-4 z-50 right-4">
			<Button
				size="sm"
				startIcon={{ icon: Pen }}
				variant="border"
				btnClasses="bg-white"
				href="{base}/apps_raw/edit/{$page.params.path}?nodraft=true">Edit</Button
			>
		</div>
	{/if}
</div>
