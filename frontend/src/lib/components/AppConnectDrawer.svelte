<script lang="ts">
	import { run } from 'svelte/legacy'

	import { createEventDispatcher } from 'svelte'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import AppConnectInner from './AppConnectInner.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'

	interface Props {
		expressOAuthSetup?: boolean
	}

	let { expressOAuthSetup = false }: Props = $props()

	let drawer: Drawer | undefined = $state()
	let resourceType = $state('')
	let step = $state(1)
	let disabled = $state(false)
	let isGoogleSignin = $state(false)
	let manual = $state(true)

	let appConnectInner: AppConnectInner | undefined = $state(undefined)

	let rtToLoad: string | undefined = $state('')
	export async function open(rt?: string) {
		rtToLoad = rt
		drawer?.openDrawer?.()
	}

	function onRtToLoadChange(rtToLoad: string | undefined) {
		appConnectInner?.open(rtToLoad)
	}

	const dispatch = createEventDispatcher()

	let darkMode: boolean = $state(false)
	run(() => {
		appConnectInner && onRtToLoadChange(rtToLoad)
	})
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
		title="Add a resource"
		on:close={() => drawer?.closeDrawer()}
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
			on:close={() => drawer?.closeDrawer()}
			on:refresh
			express={expressOAuthSetup}
		/>
		{#snippet actions()}
			<div class="flex gap-1">
				{#if step > 1}
					<Button variant="default" on:click={appConnectInner?.back ?? (() => {})}>Back</Button>
				{/if}
				{#if isGoogleSignin}
					<button {disabled} onclick={appConnectInner?.next}>
						<img
							class="h-10 w-auto object-contain"
							src={darkMode ? '/google_signin_dark.png' : '/google_signin_light.png'}
							alt="Google sign-in"
						/>
					</button>
				{:else}
					<Button variant="accent" {disabled} on:click={appConnectInner?.next ?? (() => {})}>
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
