<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import { UserService } from '$lib/gen'
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Toggle from './Toggle.svelte'
	import { generateRandomString } from '$lib/utils'
	import { globalEmailInvite } from '$lib/stores'

	export let close: (() => void) | undefined = undefined

	const dispatch = createEventDispatcher()

	let is_super_admin = false
	let password: string = generateRandomString(10)
	let name: string | undefined
	let company: string | undefined

	async function addUser() {
		await UserService.createUserGlobally({
			requestBody: {
				email: $globalEmailInvite,
				password,
				super_admin: is_super_admin,
				name,
				company
			}
		})
		sendUserToast(`Added ${$globalEmailInvite}`)
		$globalEmailInvite = ''
		password = generateRandomString(10)
		dispatch('new')
		close?.()
	}
</script>

<div class="p-4 flex flex-col gap-3 w-80">
	<h3 class="text-sm font-semibold text-emphasis">Add new user to instance</h3>
	<label class="block">
		<span class="text-xs font-semibold text-emphasis">Email</span>
		<TextInput
			inputProps={{ type: 'email', placeholder: 'email' }}
			bind:value={$globalEmailInvite}
		/>
	</label>
	<label class="block">
		<span class="text-xs font-semibold text-emphasis">Password</span>
		<TextInput bind:value={password} />
	</label>
	<label class="block">
		<span class="text-xs font-semibold text-emphasis">Name (optional)</span>
		<TextInput inputProps={{ type: 'text', placeholder: 'name (optional)' }} bind:value={name} />
	</label>
	<Toggle bind:checked={is_super_admin} options={{ right: 'Superadmin' }} />
	<Button
		variant="accent"
		size="sm"
		on:click={addUser}
		disabled={$globalEmailInvite == '' || password == undefined}
	>
		Add user to instance
	</Button>
	<div class="text-2xs text-secondary text-right">Email will be sent if SMTP configured</div>
</div>
