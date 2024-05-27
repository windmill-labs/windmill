<script lang="ts">
	import { page } from '$app/stores'

	import AppConnectInner from '$lib/components/AppConnectInner.svelte'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Button } from '$lib/components/common'
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'

	let resourceType = $page.url.searchParams.get('resource_type') ?? undefined
	let step = 1
	let disabled = false
	let isGoogleSignin = false
	let manual = false

	let appConnect: AppConnectInner | undefined = undefined

	let darkMode: boolean = false
	const workspace = $page.url.searchParams.get('workspace')

	if (workspace) {
		$workspaceStore = workspace
	}

	onMount(async () => {
		if (resourceType) {
			appConnect?.open(resourceType)
		}
	})
</script>

<DarkModeObserver bind:darkMode />

<div>
	<div class="flex flex-row-reverse w-full">
		<div class="flex gap-2">
			{#if step > 2}
				<Button variant="border" on:click={appConnect?.back}>Back</Button>
			{/if}

			{#if isGoogleSignin}
				<button {disabled} on:click={appConnect?.next}>
					<img
						class="h-10 w-auto object-contain"
						src={darkMode ? '/google_signin_dark.png' : '/google_signin_light.png'}
						alt="Google sign-in"
					/>
				</button>
			{:else}
				<Button {disabled} on:click={appConnect?.next}>
					{#if step == 2 && !manual}
						Connect
					{:else if step == 1}
						Next
					{:else}
						Save
					{/if}
				</Button>
			{/if}
		</div>
	</div>
	<AppConnectInner
		bind:this={appConnect}
		bind:step
		bind:resourceType
		bind:isGoogleSignin
		bind:disabled
		bind:manual
		on:refresh={(e) => {
			window?.parent?.postMessage({ type: 'refresh', detail: e.detail }, '*')
		}}
	/>
</div>
