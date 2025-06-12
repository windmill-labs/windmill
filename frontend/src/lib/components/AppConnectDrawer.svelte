<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import AppConnectInner from './AppConnectInner.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'

	export let expressOAuthSetup = false

	let drawer: Drawer
	let resourceType = ''
	let step = 1
	let disabled = false
	let isGoogleSignin = false
	let manual = true

	let appConnectInner: AppConnectInner | undefined = undefined

	let rtToLoad: string | undefined = ''
	export async function open(rt?: string) {
		rtToLoad = rt
		drawer.openDrawer?.()
	}

	$: appConnectInner && onRtToLoadChange(rtToLoad)

	function onRtToLoadChange(rtToLoad: string | undefined) {
		appConnectInner?.open(rtToLoad)
	}

	const dispatch = createEventDispatcher()

	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<Drawer
	bind:this={drawer}
	on:close={() => {
		step = 1
		dispatch('close')
	}}
	size="800px"
>
	<DrawerContent
		title="Add a Resource"
		on:close={drawer.closeDrawer}
		tooltip="Resources represent connections to third party systems. Learn more on how to integrate external APIs."
		documentationLink="https://www.windmill.dev/docs/integrations/integrations_on_windmill"
	>
		<AppConnectInner
			bind:this={appConnectInner}
			bind:step
			bind:resourceType
			bind:isGoogleSignin
			bind:disabled
			bind:manual
			on:close={drawer?.closeDrawer}
			on:refresh
			express={expressOAuthSetup}
		/>
		{#snippet actions()}
			<div class="flex gap-1">
				{#if step > 1}
					<Button variant="border" on:click={appConnectInner?.back ?? (() => {})}>Back</Button>
				{/if}
				{#if isGoogleSignin}
					<button {disabled} on:click={appConnectInner?.next}>
						<img
							class="h-10 w-auto object-contain"
							src={darkMode ? '/google_signin_dark.png' : '/google_signin_light.png'}
							alt="Google sign-in"
						/>
					</button>
				{:else}
					<Button {disabled} on:click={appConnectInner?.next ?? (() => {})}>
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
		{/snippet}
	</DrawerContent>
</Drawer>
