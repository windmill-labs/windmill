<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { faGithub, faGitlab, faGoogle, faMicrosoft } from '@fortawesome/free-brands-svg-icons'
	import { onMount } from 'svelte'
	import { slide } from 'svelte/transition'
	import { OauthService, UserService, WorkspaceService } from '$lib/gen'
	import { clearStores, usersWorkspaceStore, workspaceStore, userStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { getUserExt, refreshSuperadmin } from '$lib/user'
	import { Button, Skeleton } from '$lib/components/common'

	let email = $page.url.searchParams.get('email') ?? ''
	let password = $page.url.searchParams.get('password') ?? ''
	const error = $page.url.searchParams.get('error') ?? undefined
	const rd = $page.url.searchParams.get('rd')
	const providers = [
		{
			type: 'github',
			name: 'GitHub',
			icon: faGithub
		},
		{
			type: 'gitlab',
			name: 'GitLab',
			icon: faGitlab
		},
		{
			type: 'google',
			name: 'Google',
			icon: faGoogle
		},
		{
			type: 'microsoft',
			name: 'Microsoft',
			icon: faMicrosoft
		}
	] as const

	const providersType = providers.map((p) => p.type as string)

	let showPassword = false
	let logins: string[] | undefined = undefined

	async function login(): Promise<void> {
		const requestBody = {
			email,
			password
		}

		try {
			await UserService.login({ requestBody })
		} catch (err) {
			sendUserToast('Invalid credentials', true)
			return
		}

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

	function storeRedirect(provider: string) {
		if (rd) {
			try {
				localStorage.setItem('rd', rd)
			} catch (e) {
				console.error('Could not persist redirection to local storage', e)
			}
		}
		goto('/api/oauth/login/' + provider)
	}

	if (error) {
		sendUserToast(error, true)
	}
</script>

<!-- Enable submit form on enter -->
<CenteredModal title="Login">
	<div class="justify-center text-center flex flex-col">
		{#if !logins}
			{#each Array(4) as _}
				<Skeleton layout={[0.5, [2.375]]} />
			{/each}
		{:else}
			{#each providers as { type, icon, name }}
				{#if logins.includes(type)}
					<Button
						color="dark"
						variant="border"
						endIcon={{ icon }}
						btnClasses="mt-2 w-full !border-gray-300"
						on:click={() => storeRedirect(type)}
					>
						{name}
					</Button>
				{/if}
			{/each}

			{#each logins.filter((x) => !providersType.includes(x)) as login}
				<Button
					color="dark"
					variant="border"
					btnClasses="mt-2 w-full !border-gray-300"
					on:click={() => storeRedirect(login)}
				>
					{login}
				</Button>
			{/each}
		{/if}
	</div>
	<div class="center-center my-6">
		<Button
			size="xs"
			color="blue"
			variant="border"
			btnClasses="!border-none"
			on:click={() => {
				showPassword = !showPassword
			}}
		>
			Login without third-party
		</Button>
	</div>
	{#if showPassword}
		<div transition:slide>
			<p class="text-xs text-gray-500 italic pb-6">
				To get credentials without the OAuth providers above, you can send us an email at
				contact@windmill.dev or your admin owners if this instance is self-hosted and you will
				receive credentials that you can use below.
			</p>
			<label class="block pb-2">
				<span class="text-gray-700 text-sm">Email</span>
				<input type="email" bind:value={email} id="email" />
			</label>
			<label class="block ">
				<span class="text-gray-700 text-sm">Password</span>
				<input type="password" on:keyup={handleKeyUp} bind:value={password} id="password" />
			</label>
			<div class="flex justify-end pt-4">
				<Button id="login2" on:click={login}>Login</Button>
			</div>
		</div>
	{/if}
</CenteredModal>
