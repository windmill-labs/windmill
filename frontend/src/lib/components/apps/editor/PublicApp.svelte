<script lang="ts">
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import type { UserExt } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import { setContext } from 'svelte'
	import { IS_APP_PUBLIC_CONTEXT_KEY } from '../types'
	import type { AppWithLastVersion } from '$lib/gen/types.gen'
	import RawAppPreview from '$lib/components/raw_apps/RawAppPreview.svelte'
	import type { Runnable } from '$lib/components/raw_apps/rawAppPolicy'
	import { loadAppCss } from './publicAppApi'

	let {
		notExists,
		noPermission,
		jwtError,
		guestAppPath = undefined,
		onLoginSuccess,
		app,
		workspace,
		user = undefined,
		license = undefined,
		inWorkspace = false,
		hideRefreshBar = false,
		syncHashToUrl = true
	}: {
		notExists: boolean
		noPermission: boolean
		jwtError: boolean
		/** Set when this app is open to guests: signing in gets the visitor in without
		 * an account. Undefined means the ordinary "you need read access" dead end. */
		guestAppPath?: string | undefined
		onLoginSuccess: () => void
		app: (AppWithLastVersion & { value: any; workspace_id?: string }) | undefined
		workspace: string | undefined
		/** The viewer's membership in `workspace`, for the app's `ctx`. Callers pass it
		 * explicitly — `$userStore` describes the workspace the page is navigated to, which
		 * is not the app's inside an AI session's preview tab. */
		user?: UserExt | undefined
		/** The instance license, which only fades the "Powered by" badge. */
		license?: string | undefined
		/**
		 * In-workspace rendering (`/apps/get`, `/app_embed`): keep exact parity
		 * with the pre-sandbox member viewer — no "Powered by Windmill" badge, no
		 * HTML-result approval gate, column flex wrapper.
		 */
		inWorkspace?: boolean
		hideRefreshBar?: boolean
		/** Whether a raw app's route lives in the page URL's hash; see `RawAppPreview`. */
		syncHashToUrl?: boolean
	} = $props()

	// Use workspace from props or from app.workspace_id (for custom path responses)
	let effectiveWorkspace = $derived(workspace ?? app?.workspace_id)

	// On the public surfaces (untrusted distribution) runnable-authored html/svg needs
	// the viewer's approval before it renders, unless the app sandbox isolates it. The
	// in-workspace viewer renders it verbatim. See getAppMarkupTrust.
	setContext(IS_APP_PUBLIC_CONTEXT_KEY, !inWorkspace)

	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')

	if (darkMode === 'dark') {
		document.documentElement.classList.add('dark')
	} else {
		document.documentElement.classList.remove('dark')
	}
	// This route bypasses the (root) layout, so restore the variant class too.
	applyDarkModeVariant()
</script>

<!-- Only the raw-app branch is imported statically: everything else waits for app.css and
     its own chunk, see publicAppApi.ts. -->
{#if !inWorkspace}
	{#await loadAppCss() then}
		<div
			class="z-50 text-xs fixed bottom-1 right-2 {license && !isCloudHosted()
				? 'transition-opacity delay-1000 duration-1000 opacity-20 hover:delay-0 hover:opacity-100'
				: ''}"
		>
			<a href="https://windmill.dev" class="whitespace-nowrap text-primary inline-flex items-center"
				>Powered by &nbsp;<WindmillIcon />&nbsp;Windmill</a
			>
		</div>
	{/await}
{/if}

{#if notExists || noPermission}
	{#await Promise.all([loadAppCss(), import('./PublicAppAccess.svelte')]) then [, { default: PublicAppAccess }]}
		<PublicAppAccess
			{notExists}
			{jwtError}
			{guestAppPath}
			{user}
			workspace={effectiveWorkspace}
			{onLoginSuccess}
		/>
	{/await}
{:else if app}
	{#key app}
		{#if app.raw_app && effectiveWorkspace}
			<RawAppPreview
				workspace={effectiveWorkspace}
				{user}
				secret={app.bundle_secret}
				path={app.path}
				runnables={(app.value?.runnables ?? {}) as Record<string, Runnable>}
				{syncHashToUrl}
			/>
		{:else if app.raw_app && !effectiveWorkspace}
			{#await Promise.all([loadAppCss(), import('$lib/components/common/alert/Alert.svelte')]) then [, { default: Alert }]}
				<div class="px-4 mt-20">
					<Alert type="error" title="Configuration error">
						Unable to load raw app: workspace information is missing.
					</Alert>
				</div>
			{/await}
		{:else}
			{#await Promise.all([loadAppCss(), import('./PublicLowCodeApp.svelte')]) then [, { default: PublicLowCodeApp }]}
				<PublicLowCodeApp
					{app}
					workspace={effectiveWorkspace}
					{user}
					{inWorkspace}
					{hideRefreshBar}
				/>
			{/await}
		{/if}
	{/key}
{:else}
	{#await loadAppCss() then}
		<Skeleton layout={[[4], 0.5, [50]]} />
	{/await}
{/if}
