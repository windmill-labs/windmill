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
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'
	import { OauthService, UserService, WorkspaceService } from '$lib/gen'
	import { clearStores, usersWorkspaceStore, workspaceStore, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { getUserExt, refreshSuperadmin } from '$lib/user'

	let email = $page.url.searchParams.get('email') ?? ''
	let password = $page.url.searchParams.get('password') ?? ''
	const error = $page.url.searchParams.get('error') ?? undefined
	const rd = $page.url.searchParams.get('rd')

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
		if ($workspaceStore) {
			if (rd) {
				goto(decodeURI(rd))
			} else {
				goto('/')
			}
		} else {
			goto('/user/workspaces')
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

	if (error) {
		sendUserToast(error, true)
	}
</script>

<div class="min-h-screen antialiased text-gray-900">
	<!-- Enable submit form on enter -->
	<CenteredModal>
		<div class="justify-center text-center flex flex-col">
			{#if logins.includes('github')}
				<a rel="external" href="/api/oauth/login/github"
					><button class="m-auto default-button bg-black mt-2 py-2 w-full text-gray-200"
						>Github &nbsp;
						<Icon class="text-white pb-1" data={faGithub} scale={1.4} />
					</button></a
				>
			{/if}
			{#if logins.includes('gitlab')}
				<a rel="external" href="/api/oauth/login/gitlab"
					><button
						class="m-auto default-button bg-orange-400 mt-2 py-2 w-full text-black hover:bg-orange-600"
						>Gitlab &nbsp;
						<Icon class="pb-1" data={faGitlab} scale={1.4} />
					</button></a
				>
			{/if}
			{#if logins.includes('google')}
				<a rel="external" href="/api/oauth/login/google"
					><button
						class="m-auto default-button bg-gray-100 mt-2 py-2 w-full text-black hover:bg-blue-300"
						>Google &nbsp;
						<Icon class="pb-1" data={faGoogle} scale={1.4} />
					</button></a
				>
			{/if}
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
					<input bind:value={email} class="default-input" id="email" />
				</label>
				<label class="block ">
					<span class="text-gray-700">password</span>
					<input
						py-8
						type="password"
						on:keyup={handleKeyUp}
						bind:value={password}
						class="default-input"
						id="password"
					/>
				</label>
				<div class="flex flex-row-reverse  pt-4">
					<button id="login2" class="default-button" type="button" on:click={login}> Login </button>
				</div>
			</div>
		{/if}
	</CenteredModal>
</div>
