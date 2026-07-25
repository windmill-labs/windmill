<script lang="ts">
	import { User, UserRoundX } from 'lucide-svelte'
	import { enterpriseLicense, userStore } from '$lib/stores'
	import { base } from '$app/paths'
	import { page } from '$app/state'
	import Login from '$lib/components/Login.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { Alert, Skeleton } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import { getContext, onMount, setContext } from 'svelte'
	import {
		EMBED_NAV_CONTEXT_KEY,
		IS_APP_PUBLIC_CONTEXT_KEY,
		type EditorBreakpoint,
		type EmbedNav
	} from '../types'
	import { UserService, type AppWithLastVersion, type GlobalWhoamiResponse } from '$lib/gen'
	import { urlParamsToObject } from '$lib/utils'
	import { goto } from '$app/navigation'
	import AppPreview from './AppPreview.svelte'
	import RawAppPreview from '$lib/components/raw_apps/RawAppPreview.svelte'
	import type { Runnable } from '$lib/components/raw_apps/rawAppPolicy'
	import { twMerge } from 'tailwind-merge'
	import { writable } from 'svelte/store'

	let {
		notExists,
		noPermission,
		jwtError,
		onLoginSuccess,
		app,
		workspace,
		inWorkspace = false,
		hideRefreshBar = false
	}: {
		notExists: boolean
		noPermission: boolean
		jwtError: boolean
		onLoginSuccess: () => void
		app: (AppWithLastVersion & { value: any; workspace_id?: string }) | undefined
		workspace: string | undefined
		/**
		 * In-workspace rendering (`/apps/get`, `/app_embed`): keep exact parity
		 * with the pre-sandbox member viewer — no "Powered by Windmill" badge, no
		 * user overlay, no HTML-result approval gate, column flex wrapper.
		 */
		inWorkspace?: boolean
		hideRefreshBar?: boolean
	} = $props()

	// Use workspace from props or from app.workspace_id (for custom path responses)
	let effectiveWorkspace = $derived(workspace ?? app?.workspace_id)

	// On the public surfaces (untrusted distribution) runnable-authored html/svg needs
	// the viewer's approval before it renders, unless the app sandbox isolates it. The
	// in-workspace viewer renders it verbatim. See getAppMarkupTrust.
	setContext(IS_APP_PUBLIC_CONTEXT_KEY, !inWorkspace)

	// WIN-2006: inside the opaque viewer iframe, navigations to other routes
	// (navbar "app" items) must happen on the TOP page — the iframe is cookieless,
	// so navigating it would just show a login screen. PublicAppFrame provides the
	// relay; outside the opaque viewer this is undefined and goto works directly.
	const embedNav = getContext<EmbedNav | undefined>(EMBED_NAV_CONTEXT_KEY)

	const breakpoint = writable<EditorBreakpoint>('lg')

	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')

	if (darkMode === 'dark') {
		document.documentElement.classList.add('dark')
	} else {
		document.documentElement.classList.remove('dark')
	}

	let globalUser = $state<GlobalWhoamiResponse | undefined>(undefined)
	async function loadGlobalUser() {
		try {
			globalUser = await UserService.globalWhoami()
		} catch (error) {
			console.error(error)
		}
		// const user = await fetch('/api/global/user')
		// console.log(user)
	}

	onMount(() => {
		// this is to avoid loading global user if the userStore is set at loading
		setTimeout(() => {
			if ($userStore) return
			loadGlobalUser()
		}, 2000)
	})
</script>

{#if !inWorkspace}
	<div
		class="z-50 text-xs fixed bottom-1 right-2 {$enterpriseLicense && !isCloudHosted()
			? 'transition-opacity delay-1000 duration-1000 opacity-20 hover:delay-0 hover:opacity-100'
			: ''}"
	>
		<a href="https://windmill.dev" class="whitespace-nowrap text-primary inline-flex items-center"
			>Powered by &nbsp;<WindmillIcon />&nbsp;Windmill</a
		>
	</div>

	{#snippet userInfo(child)}
		<div class="flex gap-1 items-center"><User size={14} />{child}</div>
	{/snippet}

	<div class="z-50 text-2xs text-primary absolute top-3 left-2"
		>{#if $userStore}
			{@render userInfo($userStore.username)}
		{:else if globalUser}
			{@render userInfo(globalUser.email)}
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
	<div class="px-4 mt-20 w-full text-center font-bold text-xl"> This app requires read access </div>
	<div class="text-center mt-8 text-sm text-primary">
		{#if $userStore}You are logged in but have no read access to this app{:else if globalUser && effectiveWorkspace}
			You are logged in but are not a member of the workspace <span class="text-xl font-bold"
				>{effectiveWorkspace}</span
			> this app is part of
		{:else}You must be logged in and have read access to this app{/if}</div
	>
	<div class="px-2 mx-auto mt-20 max-w-xl w-full">
		{#if !jwtError}
			<Login {onLoginSuccess} popup rd={page.url.pathname + page.url.search + page.url.hash} />
		{/if}
	</div>
{:else if app}
	{#key app}
		{#if app.raw_app && effectiveWorkspace}
			<RawAppPreview
				workspace={effectiveWorkspace}
				user={$userStore}
				secret={app.bundle_secret}
				path={app.path}
				runnables={(app.value?.runnables ?? {}) as Record<string, Runnable>}
			/>
		{:else if app.raw_app && !effectiveWorkspace}
			<div class="px-4 mt-20">
				<Alert type="error" title="Configuration error">
					Unable to load raw app: workspace information is missing.
				</Alert>
			</div>
		{:else}
			<div
				class={twMerge(
					// `flex-col` matches the pre-sandbox in-workspace viewer exactly;
					// the public viewer always used a plain `flex` wrapper.
					inWorkspace
						? 'min-h-screen h-full w-full flex flex-col'
						: 'min-h-screen h-full w-full flex',
					app?.value?.['css']?.['app']?.['viewer']?.class,
					'wm-app-viewer'
				)}
				style={app?.value?.['css']?.['app']?.['viewer']?.style}
			>
				<AppPreview
					noBackend={false}
					{hideRefreshBar}
					context={{
						email: $userStore?.email,
						name: $userStore?.name,
						groups: $userStore?.groups,
						username: $userStore?.username,
						query: urlParamsToObject(page.url.searchParams, { stripReserved: true }),
						hash: page.url.hash.substring(1)
					}}
					workspace={effectiveWorkspace}
					summary={app.summary}
					app={app.value}
					appPath={app.path}
					{breakpoint}
					policy={app.policy}
					isEditor={false}
					replaceStateFn={(path) => goto(path)}
					gotoFn={(path, opt) => (embedNav ? embedNav.navigateTop(path) : goto(path, opt))}
				/>
			</div>
		{/if}
	{/key}
{:else}
	<Skeleton layout={[[4], 0.5, [50]]} />
{/if}
