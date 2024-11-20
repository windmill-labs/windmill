<script lang="ts">
	import { Pencil } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from './common/button/Button.svelte'
	import Popup from './common/popup/Popup.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import ChangeInstanceUsernameInner from './ChangeInstanceUsernameInner.svelte'
	import { UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	export let value: string | undefined
	export let email: string
	export let username: string | undefined = undefined
	export let automateUsernameCreation: boolean = false
	export let login_type: string

	let password: string = ''

	const dispatch = createEventDispatcher()

	function saveName() {
		dispatch('save', value)
	}
	async function savePassword() {
		if (password.length < 5) {
			sendUserToast('Password must be at least 5 characters long', true)
			return
		}
		await UserService.setPasswordForUser({user: email,  requestBody:{ password }})
		sendUserToast(`Password updated for ${email}`)
	}

	async function saveLoginType() {
		await UserService.setLoginTypeForUser({user: email,  requestBody:{ login_type }})
		sendUserToast(`Login type updated for ${email}`)
		dispatch('refresh')
	}
</script>

<Popup
	let:close
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
>
	<svelte:fragment slot="button">
		<Button nonCaptureEvent={true} size="xs" color="light" endIcon={{ icon: Pencil }}>Edit</Button>
	</svelte:fragment>
	<div class="flex flex-col gap-8 max-w-sm">
		{#if automateUsernameCreation && username}
			<ChangeInstanceUsernameInner {email} {username} on:renamed />
		{/if}
		<label class="block text-primary">
			<div class="pb-2 text-sm font-semibold text-primary">Name</div>
			<div class="flex w-full">
				<input
					type="text"
					bind:value
					class="!w-auto grow"
					on:click|stopPropagation={() => {}}
					on:keydown|stopPropagation
					on:keypress|stopPropagation={({ key }) => {
						if (key === 'Enter') {
							saveName()
							close(null)
						}
					}}
				/>
			</div>
			<Button
				size="xs"
				color="blue"
				buttonType="button"
				btnClasses="mt-2 "
				aria-label="Save ID"
				on:click={() => {
					saveName()
					close(null)
				}}
			>
				Update name
			</Button>
		</label>
		<label class="block text-primary">
			<div class="pb-2 text-sm font-semibold text-primary">Password</div>
			<div class="flex w-full">
				<input
					type="password"
					bind:value={password}
					class="!w-auto grow"
					on:click|stopPropagation={() => {}}
					on:keydown|stopPropagation
					on:keypress|stopPropagation={({ key }) => {
						if (key === 'Enter') {
							savePassword()
							close(null)
						}
					}}
				/>
			</div>
			<Button
				size="xs"
				color="blue"
				buttonType="button"
				btnClasses="mt-2 "
				aria-label="Save ID"
				on:click={() => {
					savePassword()
					close(null)
				}}
			>
				Update password
			</Button>
		</label>
		<label class="block text-primary">
			<div class="pb-2 text-sm font-semibold text-primary">Login type</div>
			<div class="text-xs text-secondary mb-2">
				Must match exact SSO name, "password" or "saml". Examples: password, google, saml, microsoft
			</div>

			<div class="flex w-full">
				<input
					type="text"
					bind:value={login_type}
					class="!w-auto grow"
					on:click|stopPropagation={() => {}}
					on:keydown|stopPropagation
					on:keypress|stopPropagation={({ key }) => {
						if (key === 'Enter') {
							saveLoginType()
							close(null)
						}
					}}
				/>
			</div>
			<Button
				size="xs"
				color="blue"
				buttonType="button"
				btnClasses="mt-2 "
				aria-label="Save login type"
				on:click={() => {
					saveLoginType()
					close(null)
				}}
			>
				Update login type
			</Button>
		</label>
	</div>
</Popup>
