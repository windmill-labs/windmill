<script lang="ts">
	import { base } from '$app/paths'
	import { page } from '$app/state'
	import Login from '$lib/components/Login.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { UserService, type GlobalWhoamiResponse } from '$lib/gen'
	import type { UserExt } from '$lib/stores'
	import { dropPrefetched } from './publicAppApi'

	let {
		notExists,
		jwtError,
		guestAppPath,
		user,
		workspace,
		onLoginSuccess
	}: {
		notExists: boolean
		jwtError: boolean
		guestAppPath: string | undefined
		user: UserExt | undefined
		workspace: string | undefined
		onLoginSuccess: () => void
	} = $props()

	let globalUser = $state<GlobalWhoamiResponse | undefined>(undefined)
	async function loadGlobalUser() {
		try {
			globalUser = await UserService.globalWhoami()
		} catch (error) {
			console.error(error)
		}
	}

	// Only the no-access page reads it, to tell a signed-in non-member which
	// workspace the app belongs to.
	$effect(() => {
		if (!notExists && !guestAppPath && !user && !globalUser) {
			loadGlobalUser()
		}
	})
</script>

{#if notExists}
	<div class="px-4 mt-20"
		><Alert type="error" title="Not found"
			>There was an error loading the app, is the url correct? <a href={base}>Go to Windmill</a>
		</Alert></div
	>
{:else}
	{#if guestAppPath && !user}
		<div class="px-4 mt-20 w-full text-center font-bold text-xl"> Sign in to open this app </div>
		<div class="text-center mt-8 text-sm text-primary">
			You do not need a Windmill account. Signing in lets you open this app and nothing else.
		</div>
	{:else}
		<div class="px-4 mt-20 w-full text-center font-bold text-xl">
			This app requires read access
		</div>
		<div class="text-center mt-8 text-sm text-primary">
			{#if user}You are logged in but have no read access to this app{:else if globalUser && workspace}
				You are logged in but are not a member of the workspace <span class="text-xl font-bold"
					>{workspace}</span
				> this app is part of
			{:else}You must be logged in and have read access to this app{/if}</div
		>
	{/if}
	<div class="px-2 mx-auto mt-20 max-w-xl w-full">
		{#if !jwtError}
			<Login
				onLoginSuccess={() => {
					dropPrefetched()
					onLoginSuccess()
				}}
				popup
				guestApp={guestAppPath}
				rd={page.url.pathname + page.url.search + page.url.hash}
			/>
		{/if}
	</div>
{/if}
