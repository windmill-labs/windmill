<script lang="ts">
	import { browser } from '$app/environment'
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import type { EditorBreakpoint } from '$lib/components/apps/types'

	import { Alert, Skeleton } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import { AppService, AppWithLastVersion, GlobalUserInfo, UserService } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import github from 'svelte-highlight/styles/github'
	import { writable } from 'svelte/store'

	let app: AppWithLastVersion | undefined = undefined
	let user: GlobalUserInfo | undefined = undefined
	let notExists = false

	async function loadApp() {
		try {
			app = await AppService.getPublicAppBySecret({
				workspace: $page.params.workspace,
				path: $page.params.secret
			})
		} catch (e) {
			notExists = true
		}
	}

	async function loadUser() {
		try {
			user = await UserService.globalWhoami()
		} catch (e) {}
	}
	if (browser) {
		loadApp()
		loadUser()
	}

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

<svelte:head>
	{@html github}
</svelte:head>

<div class="z-50 text-xs fixed bottom-1 right-2 ">
	<a href="https://windmill.dev" class="whitespace-nowrap text-gray-500 inline-flex items-center"
		>Powered by &nbsp;<WindmillIcon />&nbsp;Windmill</a
	>
</div>
<div class="z-50 text-xs text-gray-500 fixed top-1 left-2">
	<div>
		{#if user}
			Logged in as {user.email}
		{:else}
			Not logged in
		{/if}
	</div>
	<a class="text-blue-400" href="/">Go to app</a>
</div>
{#if notExists}
	<div class="px-4 mt-20"
		><Alert type="error" title="Not found"
			>There was an error loading the app. Either it does not exist at this url or its visibility
			has changed to not be public anymore. <a href="/">Go to app</a>
		</Alert></div
	>
{:else if app}
	<div class="border rounded-md p-2 w-full">
		<AppPreview
			noBackend={false}
			context={{
				email: $userStore?.email,
				username: $userStore?.username,
				query: Object.fromEntries($page.url.searchParams.entries())
			}}
			workspace={$page.params.workspace}
			summary={app.summary}
			app={app.value}
			appPath={app.path}
			{breakpoint}
			policy={app.policy}
			isEditor={false}
		/>
	</div>
{:else}
	<Skeleton layout={[[4], 0.5, [50]]} />
{/if}
