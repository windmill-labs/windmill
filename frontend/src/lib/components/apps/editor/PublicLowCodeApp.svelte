<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/state'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import type { AppWithLastVersion } from '$lib/gen'
	import { enterpriseLicense, userStore, type UserExt } from '$lib/stores'
	import { urlParamsToObject } from '$lib/utils'
	import { getContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import { EMBED_NAV_CONTEXT_KEY, type EditorBreakpoint, type EmbedNav } from '../types'
	import { loadAppPreview } from './loadAppPreview'

	let {
		app,
		workspace,
		user,
		license,
		inWorkspace,
		hideRefreshBar
	}: {
		app: AppWithLastVersion & { value: any }
		workspace: string | undefined
		user: UserExt | undefined
		license: string | undefined
		inWorkspace: boolean
		hideRefreshBar: boolean
	} = $props()

	// The public routes hold the viewer and license as props, while low-code components
	// read them from the stores (the license gates the app's custom CSS, for one).
	$effect(() => {
		if (inWorkspace) return
		userStore.set(user)
		if (license) enterpriseLicense.set(license)
	})

	// WIN-2006: inside the opaque viewer iframe, navigations to other routes
	// (navbar "app" items) must happen on the TOP page — the iframe is cookieless,
	// so navigating it would just show a login screen. PublicAppFrame provides the
	// relay; outside the opaque viewer this is undefined and goto works directly.
	const embedNav = getContext<EmbedNav | undefined>(EMBED_NAV_CONTEXT_KEY)

	const breakpoint = writable<EditorBreakpoint>('lg')
</script>

<div
	class={twMerge(
		// `flex-col` matches the pre-sandbox in-workspace viewer exactly;
		// the public viewer always used a plain `flex` wrapper. `min-h-full`, not
		// `h-full`: the box has to reach the bottom of what the host gives it so the
		// app's own background covers the window, and still grow with a grid taller
		// than that. A viewport floor instead would overhang the page box by the
		// height of the page header band.
		inWorkspace ? 'min-h-full w-full flex flex-col' : 'min-h-full w-full flex',
		app?.value?.['css']?.['app']?.['viewer']?.class,
		'wm-app-viewer'
	)}
	style={app?.value?.['css']?.['app']?.['viewer']?.style}
>
	{#await loadAppPreview()}
		<Skeleton layout={[[4], 0.5, [50]]} />
	{:then Module}
		<Module.default
			noBackend={false}
			{hideRefreshBar}
			context={{
				email: user?.email,
				name: user?.name,
				groups: user?.groups,
				username: user?.username,
				query: urlParamsToObject(page.url.searchParams, { stripReserved: true }),
				hash: page.url.hash.substring(1)
			}}
			{workspace}
			summary={app.summary}
			app={app.value}
			appPath={app.path}
			{breakpoint}
			policy={app.policy}
			isEditor={false}
			replaceStateFn={(path) => goto(path)}
			gotoFn={(path, opt) => (embedNav ? embedNav.navigateTop(path) : goto(path, opt))}
		/>
	{:catch}
		<div class="px-4 mt-20 w-full">
			<Alert type="error" title="Could not load the app">Reload the page to try again.</Alert>
		</div>
	{/await}
</div>
