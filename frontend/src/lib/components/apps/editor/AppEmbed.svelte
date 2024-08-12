<script lang="ts">
	import { BROWSER } from 'esm-env'
	import { base } from '$lib/base'
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

	export let workspace: string
	export let secret: string | undefined = undefined
	export let path: string | undefined = undefined
	export let replaceStateFn: (path: string) => void = (path: string) =>
		window.history.replaceState(null, '', path)
	export let gotoFn: (path: string, opt?: Record<string, any> | undefined) => void = (
		path: string,
		opt?: Record<string, any>
	) => window.history.pushState(null, '', path)

	export let hideUser = false

	let app: (AppWithLastVersion & { value: any }) | undefined = undefined
	let notExists = false
	let noPermission = false
	let neitherSecretNorPath = !secret && !path
	setContext(IS_APP_PUBLIC_CONTEXT_KEY, true)

	async function loadApp() {
		try {
			neitherSecretNorPath = !secret && !path
			if (neitherSecretNorPath) {
				throw new Error('No secret or path provided')
			}
			app = secret
				? await AppService.getPublicAppBySecret({
						workspace: workspace,
						path: secret
				  })
				: await AppService.getAppByPath({
						workspace: workspace,
						path: path!
				  })
			noPermission = false
			notExists = false
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
		loadUser().then(() => {
			loadApp()
		})
	}

	async function loadUser() {
		try {
			userStore.set(await getUserExt(workspace))
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

{#if !hideUser}
	<div class="z-50 text-2xs text-tertiary absolute top-3 left-2"
		>{#if $userStore}
			<div class="flex gap-1 items-center"><User size={14} />{$userStore.username}</div>
		{:else}<UserRoundX size={14} />{/if}
	</div>
{/if}

{#if notExists}
	<div class="px-4 mt-20"
		><Alert type="error" title="Not found"
			>There was an error loading the app, is the url correct? <a href={base}>Go to Windmill</a>
		</Alert></div
	>
{:else if noPermission}
	<div class="px-4 mt-20 w-full text-center font-bold text-xl"
		>{#if $userStore}You are logged in but have no read access for this app{:else}You must be logged
			in and have read access for this app{/if}</div
	>
	<div class="px-2 mx-auto mt-20 max-w-xl w-full">
		<Login
			loginPasswordRequireEEOnPublicApps
			on:login={() => {
				// window.location.reload()
				loadUser().then(() => {
					loadApp()
				})
				app = app
			}}
			popup
			rd={window.location.href}
		/>
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
				username={$userStore?.username ?? 'anonymous'}
				email={$userStore?.email ?? 'anonymous'}
				groups={$userStore?.groups ?? []}
				{workspace}
				summary={app.summary}
				app={app.value}
				appPath={app.path}
				{breakpoint}
				author={app.policy.on_behalf_of_email ?? ''}
				isEditor={false}
				{replaceStateFn}
				{gotoFn}
			/>
		</div>
	{/key}
{:else}
	<Skeleton layout={[[4], 0.5, [50]]} />
{/if}
