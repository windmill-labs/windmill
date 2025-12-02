<script lang="ts">
	import { goto } from '$app/navigation'
	import { base } from '$lib/base'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import { Button, Skeleton } from '$lib/components/common'
	import { AppService, type AppWithLastVersion } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canWrite, urlParamsToObject } from '$lib/utils'
	import { Pen } from 'lucide-svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import { page } from '$app/state'

	let app: (AppWithLastVersion & { value: any }) | undefined = $state(undefined)
	let can_write = $state(false)

	async function loadApp() {
		app = await AppService.getAppLiteByPath({
			workspace: $workspaceStore!,
			path: page.params.path ?? ''
		})
		can_write = canWrite(app?.path, app?.extra_perms!, $userStore)
	}

	$effect(() => {
		if ($workspaceStore && page.params.path) {
			if (app && page.params.path === app.path) {
				console.log('App already loaded')
			} else {
				loadApp()
			}
		}
	})

	const breakpoint = writable<EditorBreakpoint>('lg')

	const hideRefreshBar = page.url.searchParams.get('hideRefreshBar') === 'true'
	const hideEditBtn = page.url.searchParams.get('hideEditBtn') === 'true'
</script>

{#if app}
	{#key app}
		<div
			class={twMerge(
				'min-h-screen h-full w-full flex flex-col',
				app?.value.css?.['app']?.['viewer']?.class,
				'wm-app-viewer'
			)}
			style={app?.value.css?.['app']?.['viewer']?.style}
		>
			<AppPreview
				context={{
					email: $userStore?.email,
					name: $userStore?.name,
					username: $userStore?.username,
					groups: $userStore?.groups,
					query: urlParamsToObject(page.url.searchParams),
					hash: page.url.hash.substring(1)
				}}
				workspace={$workspaceStore ?? ''}
				summary={app.summary}
				app={app.value}
				appPath={page.params.path}
				{breakpoint}
				policy={app.policy}
				isEditor={false}
				noBackend={false}
				{hideRefreshBar}
				replaceStateFn={(path) => {
					goto(path)
				}}
				gotoFn={(path, opt) => {
					goto(path, opt)
				}}
			/>
			{#if can_write && !hideEditBtn}
				<div id="app-edit-btn" class="absolute bottom-4 z-50 right-4">
					<Button
						size="sm"
						startIcon={{ icon: Pen }}
						variant="subtle"
						href="{base}/apps/edit/{app.path}?nodraft=true">Edit</Button
					>
				</div>
			{/if}
		</div>
	{/key}
{:else}
	<Skeleton layout={[10]} />
{/if}
