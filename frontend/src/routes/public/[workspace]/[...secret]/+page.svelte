<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import { IS_APP_PUBLIC_CONTEXT_KEY, type EditorBreakpoint } from '$lib/components/apps/types'

	import { Alert, Skeleton } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import { AppService, AppWithLastVersion } from '$lib/gen'
	import { userStore } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'

	import { setContext } from 'svelte'
	import github from 'svelte-highlight/styles/github'
	import { writable } from 'svelte/store'
	import { setLicense } from '$lib/enterpriseUtils'

	let app: AppWithLastVersion | undefined = undefined
	let notExists = false

	setContext(IS_APP_PUBLIC_CONTEXT_KEY, true)

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

	if (BROWSER) {
		setLicense()
		loadApp()
	}

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

<svelte:head>
	{@html github}
</svelte:head>

<div class="z-50 text-xs fixed bottom-1 right-2">
	<a href="https://windmill.dev" class="whitespace-nowrap text-tertiary inline-flex items-center"
		>Powered by &nbsp;<WindmillIcon />&nbsp;Windmill</a
	>
</div>

{#if notExists}
	<div class="px-4 mt-20"
		><Alert type="error" title="Not found"
			>There was an error loading the app. Either it does not exist at this url or its visibility
			has changed to not be public anymore. <a href="/">Go to app</a>
		</Alert></div
	>
{:else if app}
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
				noBackend={false}
				context={{
					email: $userStore?.email,
					groups: $userStore?.groups,
					username: $userStore?.username,
					query: Object.fromEntries($page.url.searchParams.entries()),
					hash: $page.url.hash
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
	{/key}
{:else}
	<Skeleton layout={[[4], 0.5, [50]]} />
{/if}
