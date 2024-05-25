<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import { apiTokenApps, forceSecretValue, linkedSecretValue } from './app_connect'
	import AppConnectInner from './AppConnectInner.svelte'
	import DarkModeObserver from './DarkModeObserver.svelte'

	export let newPageOAuth = false

	let value: string = ''

	let drawer: Drawer
	let resourceType = ''
	let step = 1
	let isValid = true

	let appConnectInner: AppConnectInner | undefined = undefined

	export async function open(rt?: string) {
		appConnectInner?.open(rt)
		drawer.openDrawer?.()
	}

	export function openFromOauth(rt: string) {
		appConnectInner?.openFromOauth(rt)
		drawer.openDrawer?.()
	}

	const dispatch = createEventDispatcher()

	$: isGoogleSignin =
		step == 1 &&
		(resourceType == 'google' ||
			resourceType == 'gmail' ||
			resourceType == 'gcal' ||
			resourceType == 'gdrive' ||
			resourceType == 'gsheets')

	$: disabled =
		(step == 1 && resourceType == '') ||
		(step == 2 &&
			value == '' &&
			args &&
			args['token'] == '' &&
			args['password'] == '' &&
			args['api_key'] == '' &&
			args['key'] == '' &&
			linkedSecret != undefined) ||
		(step == 3 && pathError != '') ||
		!isValid

	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<Drawer
	bind:this={drawer}
	on:close={() => {
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
			bind:isValid
			{newPageOAuth}
		/>
		<div slot="actions" class="flex gap-1">
			{#if step > 1 && !no_back}
				<Button variant="border" on:click={appConnectInner?.back}>Back</Button>
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
				<Button {disabled} on:click={appConnectInner?.next}>
					{#if step == 1 && !manual}
						Connect
					{:else if step == 1 && manual}
						Next
					{:else}
						Save
					{/if}
				</Button>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
