<script context="module">
	export function load() {
		return {
			stuff: { title: 'Login' }
		}
	}
</script>

<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { faGithub, faGitlab, faGoogle } from '@fortawesome/free-brands-svg-icons'
	import { onMount } from 'svelte'
	import { slide } from 'svelte/transition'
	import { OauthService, UserService, WorkspaceService } from '$lib/gen'
	import { clearStores, usersWorkspaceStore, workspaceStore, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { getUserExt, refreshSuperadmin } from '$lib/user'
	import { Button } from '$lib/components/common'

	let email = $page.url.searchParams.get('email') ?? ''
	let password = $page.url.searchParams.get('password') ?? ''
	const error = $page.url.searchParams.get('error') ?? undefined
	const rd = $page.url.searchParams.get('rd')
	const providers = [
		{
			type: 'github',
			name: 'Github',
			icon: faGithub
		},
		{
			type: 'gitlab',
			name: 'Gitlab',
			icon: faGitlab
		},
		{
			type: 'google',
			name: 'Google',
			icon: faGoogle
		}
	] as const

	let showPassword = false
	let logins: string[] = []

	async function login(): Promise<void> {
		const requestBody = {
			email,
			password
		}

		await UserService.login({ requestBody })

		// Once logged in, we can fetch the workspaces
		$usersWorkspaceStore = await WorkspaceService.listUserWorkspaces()
		// trigger a reload of the user
		if ($workspaceStore) {
			$userStore = await getUserExt($workspaceStore)
		}

		// Finally, we check whether the user is a superadmin
		refreshSuperadmin()
		redirectUser()
	}

	function redirectUser() {
		if (rd?.startsWith('http')) {
			goto(rd)
			return
		}
		if ($workspaceStore) {
			goto(rd ?? '/')
		} else {
			goto(`/user/workspaces${rd ? `?rd=${encodeURIComponent(rd)}` : ''}`)
		}
	}

	async function loadLogins() {
		logins = await OauthService.listOAuthLogins()
		showPassword = logins.length == 0
	}

	onMount(async () => {
		try {
			loadLogins()
			await UserService.getCurrentEmail()
			redirectUser()
		} catch {
			clearStores()
		}
	})

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key

		if (key === 'Enter') {
			event.preventDefault()
			login()
		}
	}

	function storeRedirect(provider: typeof providers[number]['type']) {
		if (rd) {
			localStorage.setItem('rd', rd)
		}
		goto('/api/oauth/login/' + provider)
	}

	if (error) {
		sendUserToast(error, true)
	}
</script>

<div class="min-h-screen antialiased text-gray-900">
	<!-- Enable submit form on enter -->
	<CenteredModal title="Login">
		<div class="justify-center text-center flex flex-col">
			{#each providers as { type, icon, name }}
				{#if logins.includes(type)}
					<Button
						color="dark"
						variant="border"
						endIcon={{ icon }}
						btnClasses="mt-2 w-full"
						on:click={() => storeRedirect(type)}
					>
						{name}
					</Button>
				{/if}
			{/each}
		</div>
		<div class="flex flex-row-reverse w-full">
			<button
				class="my-6 text-xs text-blue-400 m-auto"
				id="showPassword"
				on:click={() => {
					showPassword = !showPassword
				}}>login without third-party</button
			>
		</div>
		{#if showPassword}
			<div transition:slide>
				<p class="text-xs text-gray-400 italic my-2">
					To get credentials without the OAuth providers above, you can send us an email at
					contact@windmill.dev or your admin owners if this instance is self-hosted and you will
					receive credentials that you can use below.
				</p>
				<label class="block pb-2">
					<span class="text-gray-700">email</span>
					<input type="text" bind:value={email} class="mt-1" id="email" />
				</label>
				<label class="block ">
					<span class="text-gray-700">password</span>
					<input
						type="password"
						on:keyup={handleKeyUp}
						bind:value={password}
						class="mt-1"
						id="password"
					/>
				</label>
				<div class="flex justify-end pt-4">
					<Button id="login2" on:click={login}>Login</Button>
				</div>
			</div>
		{/if}
	</CenteredModal>
</div>
