<script lang="ts">
	import { onMount } from 'svelte'
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'
	import { UserService } from '$lib/gen'
	import { OpenAPI } from '$lib/gen/core/OpenAPI'
	import { sendUserToast } from '$lib/toast'

	// The /test_dev pages render the whitelabel SDK components, which authenticate
	// via a bearer token + workspace exactly like the React SDK's initializeClients
	// (OpenAPI.TOKEN + workspaceStore + userStore). These routes are outside the
	// (logged) layout, so none of that is wired automatically — this header is the
	// single place to log in, set the token, and pick the workspace for all of them.

	const TOKEN_KEY = 'test_dev_token'
	const WORKSPACE_KEY = 'workspace'

	let workspace = $state('admins')
	let email = $state('admin@windmill.dev')
	let password = $state('changeme')
	let token = $state('')
	let loading = $state(false)

	function persistToken(t: string | undefined) {
		try {
			if (t) localStorage.setItem(TOKEN_KEY, t)
			else localStorage.removeItem(TOKEN_KEY)
		} catch {}
	}

	function applyToken(t: string | undefined) {
		token = t ?? ''
		OpenAPI.TOKEN = t || undefined
		persistToken(t)
	}

	async function loadUser() {
		if (!$workspaceStore) return
		$userStore = await getUserExt($workspaceStore)
	}

	function applyWorkspace(w: string) {
		if (!w) return
		workspaceStore.set(w)
		try {
			localStorage.setItem(WORKSPACE_KEY, w)
		} catch {}
	}

	async function login() {
		if (loading) return
		loading = true
		try {
			// Returns the session token as plaintext; the bearer token is the
			// preferred SDK auth method (the cookie is browser convenience only).
			const t = await UserService.login({ requestBody: { email, password } })
			applyToken(t)
			applyWorkspace(workspace)
			await loadUser()
			sendUserToast(`Logged in as ${$userStore?.username ?? email}`)
		} catch (err: any) {
			sendUserToast(`Login failed: ${err?.body ?? err?.message ?? err}`, true)
		} finally {
			loading = false
		}
	}

	async function setTokenManually() {
		if (!token) {
			sendUserToast('Enter a token first', true)
			return
		}
		applyToken(token)
		applyWorkspace(workspace)
		await loadUser()
		sendUserToast('Token set')
	}

	function logout() {
		applyToken(undefined)
		$userStore = undefined
		sendUserToast('Token cleared')
	}

	// Restore a persisted session on mount so a reload stays authenticated.
	onMount(() => {
		let storedWs: string | null = null
		let storedToken: string | null = null
		try {
			storedWs = localStorage.getItem(WORKSPACE_KEY)
			storedToken = localStorage.getItem(TOKEN_KEY)
		} catch {}
		if (storedWs) workspace = storedWs
		else if ($workspaceStore) workspace = $workspaceStore
		if (storedToken) applyToken(storedToken)
		applyWorkspace(workspace)
		if (storedToken) void loadUser()
	})
</script>

<div
	class="sticky top-0 z-50 flex flex-wrap items-end gap-3 border-b bg-surface-secondary px-4 py-2 text-xs"
>
	<label class="flex flex-col gap-1">
		workspace
		<TextInput bind:value={workspace} size="sm" inputProps={{ placeholder: 'admins' }} />
	</label>
	<label class="flex flex-col gap-1">
		email
		<TextInput bind:value={email} size="sm" inputProps={{ placeholder: 'admin@windmill.dev' }} />
	</label>
	<label class="flex flex-col gap-1">
		password
		<TextInput bind:value={password} size="sm" inputProps={{ type: 'password' }} />
	</label>
	<Button {loading} size="xs" onclick={login}>Log in</Button>

	<label class="flex flex-col gap-1 grow min-w-48">
		token
		<TextInput
			bind:value={token}
			size="sm"
			inputProps={{ placeholder: 'paste a token to set manually' }}
		/>
	</label>
	<Button variant="default" size="xs" onclick={setTokenManually}>Set token</Button>
	<Button variant="subtle" size="xs" onclick={logout}>Clear</Button>

	<div class="flex items-center gap-1 ml-auto whitespace-nowrap">
		{#if $userStore}
			<span class="text-green-600 font-semibold">●</span>
			<span class="text-secondary">{$userStore.username} @ {$workspaceStore}</span>
		{:else if token}
			<span class="text-orange-500 font-semibold">●</span>
			<span class="text-secondary">token set, no user</span>
		{:else}
			<span class="text-tertiary font-semibold">●</span>
			<span class="text-tertiary">not authenticated</span>
		{/if}
	</div>
</div>
