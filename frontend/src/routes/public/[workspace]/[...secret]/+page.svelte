<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { page } from '$app/stores'
	import AppPreview from '$lib/components/apps/editor/AppPreview.svelte'
	import { IS_APP_PUBLIC_CONTEXT_KEY, type EditorBreakpoint } from '$lib/components/apps/types'

	import { Alert, Skeleton } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import { AppService, type AppWithLastVersion } from '$lib/gen'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'

	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { setLicense } from '$lib/enterpriseUtils'
	import { isCloudHosted } from '$lib/cloud'
	import Login from '$lib/components/Login.svelte'
	import { getUserExt } from '$lib/user'
	import { User, UserRoundX } from 'lucide-svelte'
	import { replaceState } from '$app/navigation'

	let app: (AppWithLastVersion & { value: any }) | undefined = undefined
	let notExists = false
	let noPermission = false
	setContext(IS_APP_PUBLIC_CONTEXT_KEY, true)

	async function loadApp() {
		try {
			app = await AppService.getPublicAppBySecret({
				workspace: $page.params.workspace,
				path: $page.params.secret
			})
		} catch (e) {
			if (e.status == 401) {
				noPermission = true
			} else {
				notExists = true
			}
		}
	}

	if (BROWSER) {
		setLicense()
		loadApp()
		loadUser()
	}

	async function loadUser() {
		try {
			userStore.set(await getUserExt($page.params.workspace))
		} catch (e) {
			console.warn('Anonymous user')
		}
	}

	const breakpoint = writable<EditorBreakpoint>('lg')

	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')

	if (darkMode === 'dark') {
		document.documentElement.classList.add('dark')
	} else {
		document.documentElement.classList.remove('dark')
	}
</script>

<div
	class="z-50 text-xs fixed bottom-1 right-2 {$enterpriseLicense && !isCloudHosted()
		? 'transition-opacity delay-1000 duration-1000 opacity-20 hover:delay-0 hover:opacity-100'
		: ''}"
>
	<a href="https://windmill.dev" class="whitespace-nowrap text-tertiary inline-flex items-center"
		>Powered by &nbsp;<WindmillIcon />&nbsp;Windmill</a
	>
</div>

<div class="z-50 text-2xs text-tertiary absolute top-3 left-2"
	>{#if $userStore}
		<div class="flex gap-1 items-center"><User size={14} />{$userStore.username}</div>
	{:else}<UserRoundX size={14} />{/if}
</div>

{#if notExists}
	<div class="px-4 mt-20"
		><Alert type="error" title="Not found"
			>There was an error loading the app, is the url correct? <a href="/">Go to Windmill</a>
		</Alert></div
	>
{:else if noPermission}
	<div class="px-4 mt-20 w-full text-center font-bold text-xl"
		>{#if $userStore}You are logged in but have no read access for this app{:else}You must be logged
			in and have read access for this app{/if}</div
	>
	<div class="px-2 mx-auto mt-20 max-w-xl w-full">
		<Login rd={$page.url.toString()} />
	</div>
{:else if app}
	{#key app}
		<div
			class={twMerge(
				'min-h-screen h-full w-full',
				app?.value?.['css']?.['app']?.['viewer']?.class,
				'wm-app-viewer'
			)}
			style={app?.value?.['css']?.['app']?.['viewer']?.style}
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
				replaceStateFn={(path) => replaceState(path, $page.state)}
			/>
		</div>
	{/key}
{:else}
	<Skeleton layout={[[4], 0.5, [50]]} />
{/if}
