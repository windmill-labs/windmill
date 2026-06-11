<script lang="ts">
	/*
	 * WIN-2006: in-workspace app viewer. The app is untrusted markup/JS, so it must
	 * not run with the member's full session. This page is the embedder (keeps the
	 * workspace chrome + Edit button) and renders the app inside an opaque, scoped
	 * iframe pointing at the cookieless `/app_embed` viewer route — the same
	 * PublicAppFrame machinery the public viewer uses, by path instead of by secret.
	 */
	import { base } from '$lib/base'
	import PublicApp from '$lib/components/apps/editor/PublicApp.svelte'
	import PublicAppFrame from '$lib/components/apps/editor/PublicAppFrame.svelte'
	import { Button, Skeleton } from '$lib/components/common'
	import { AppService, OpenAPI } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { canWrite } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { Pen } from 'lucide-svelte'
	import { page } from '$app/state'

	let app: any = $state(undefined)
	let notExists = $state(false)
	let noPermission = $state(false)
	let canWriteApp = $state(false)
	let refresh: (() => void) | undefined

	let workspace = $derived($workspaceStore ?? '')
	let path = $derived(page.params.path ?? '')
	// The opaque iframe loads the dedicated cookieless, chrome-less viewer route.
	let viewerUrl = $derived(`${base}/app_embed/${workspace}/${path}`)

	const hideEditBtn = page.url.searchParams.get('hideEditBtn') === 'true'

	// Embedder side: mint a scoped embed token (by path) from the member's session.
	async function fetchEmbedToken(): Promise<{ token?: string }> {
		const headers: Record<string, string> = {}
		if (typeof OpenAPI.TOKEN === 'string' && OpenAPI.TOKEN) {
			headers['Authorization'] = `Bearer ${OpenAPI.TOKEN}`
		}
		const res = await fetch(`${OpenAPI.BASE}/w/${workspace}/apps/embed_token/p/${path}`, {
			headers
		})
		if (!res.ok) {
			const err: any = new Error('Failed to fetch embed token')
			err.status = res.status
			throw err
		}
		return await res.json()
	}

	// Viewer side — only for the unsandboxed direct-render (legacy / consented
	// disable-sandbox); the sandboxed case loads inside the opaque /app_embed iframe.
	async function loadApp() {
		try {
			userStore.set(await getUserExt(workspace))
		} catch (e) {
			console.warn('Anonymous user')
		}
		try {
			app = await AppService.getAppByPath({ workspace, path })
			noPermission = false
			notExists = false
		} catch (e: any) {
			if (e.status == 401) refresh?.()
			else if (e.status == 403) noPermission = true
			else notExists = true
		}
	}

	// Edit button: determine write access on this real-origin page (cookie).
	async function loadPerms() {
		try {
			const lite: any = await AppService.getAppLiteByPath({ workspace, path })
			canWriteApp = canWrite(lite?.path, lite?.extra_perms ?? {}, $userStore)
		} catch (_) {
			canWriteApp = false
		}
	}

	$effect(() => {
		if (workspace && path) loadPerms()
	})
</script>

<!-- Wait for the active workspace before mounting the embedder: it's needed to
     mint the token and to build the viewer iframe URL, and the store is set
     asynchronously by the (logged) layout. -->
{#if $workspaceStore && path}
	<PublicAppFrame
		{fetchEmbedToken}
		{viewerUrl}
		onViewerReady={(_token, requestTokenRefresh) => {
			refresh = requestTokenRefresh
			loadApp()
		}}
	>
		{#snippet viewer()}
			<PublicApp
				{app}
				{workspace}
				{notExists}
				{noPermission}
				jwtError={false}
				onLoginSuccess={() => loadApp()}
			></PublicApp>
		{/snippet}
	</PublicAppFrame>
{:else}
	<Skeleton layout={[10]} />
{/if}

{#if canWriteApp && !hideEditBtn}
	<div id="app-edit-btn" class="absolute bottom-4 z-50 right-4">
		<Button
			size="sm"
			startIcon={{ icon: Pen }}
			variant="subtle"
			href="{base}/apps/edit/{path}?nodraft=true">Edit</Button
		>
	</div>
{/if}
