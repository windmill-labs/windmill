<script lang="ts">
	import { goto } from '$app/navigation'

	import { UserService } from '../../gen'
	import { refreshSuperadmin, sendUserToast } from '../../utils'
	import { page } from '$app/stores'
	import { userStore, usersWorkspaceStore, workspaceStore } from '../../stores'
	import CenteredModal from './CenteredModal.svelte'
	import Icon from 'svelte-awesome'
	import { faGithub } from '@fortawesome/free-brands-svg-icons'
	import { slide } from 'svelte/transition'
	import { onMount } from 'svelte'

	let email = $page.url.searchParams.get('email') ?? ''
	let password = $page.url.searchParams.get('password') ?? ''
	const error = $page.url.searchParams.get('error') ?? undefined
	const rd = $page.url.searchParams.get('rd')

	let showPassword = false

	async function login(): Promise<void> {
		try {
			await UserService.login({
				requestBody: {
					email: email,
					password
				}
			})
			refreshSuperadmin()
			if (rd) {
				goto(decodeURI(rd))
			} else {
				goto('/user/workspaces')
			}
		} catch (err) {
			sendUserToast(`Cannot login: ${err.body}`, true)
		}
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key || event.keyCode
		if (key === 13 || key === 'Enter') {
			event.preventDefault()
			login()
		}
	}

	async function redirectIfLoggedIn() {
		try {
			await UserService.getCurrentEmail()
			goto('/')
		} catch {
			usersWorkspaceStore.set(undefined)
			workspaceStore.set(undefined)
			userStore.set(undefined)
		}
	}

	onMount(() => {
		setTimeout(redirectIfLoggedIn, 1000)
	})
</script>

<!-- Enable submit form on enter -->
<CenteredModal subtitle={error}>
	<div class="justify-center text-center flex flex-col">
		<span class="text-xs text-gray-600">Currently only signup through Github is supported</span>
		<a rel="external" href="/api/oauth/login/github"
			><button class="m-auto default-button bg-black mt-2 py-2 w-96"
				>Signup or login with Github &nbsp;
				<Icon class="text-white pb-1" data={faGithub} scale={1.4} />
			</button></a
		>
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
				Signup without Github is not supported currently but if you do not want to use the github
				login flow, you can send us an email at contact@windmill.dev and you will receive
				credentials that you can use below.
			</p>
			<label class="block pb-2">
				<span class="text-gray-700">email</span>
				<input bind:value={email} class="default-input" />
			</label>
			<label class="block ">
				<span class="text-gray-700">password</span>
				<input
					py-8
					type="password"
					on:keyup={handleKeyUp}
					bind:value={password}
					class="default-input"
				/>
			</label>
			<div class="flex flex-row-reverse  pt-4">
				<button class="default-button" type="button" on:click={login}> Login </button>
			</div>
		</div>
	{/if}
</CenteredModal>
