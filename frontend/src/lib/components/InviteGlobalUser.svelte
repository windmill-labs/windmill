<script lang="ts">
	import { sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { UserService } from '$lib/gen'
	import { Button } from './common'
	import Toggle from './Toggle.svelte'

	const dispatch = createEventDispatcher()

	let email: string
	let is_super_admin = false
	let password: string
	let name: string | undefined
	let company: string | undefined

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addUser()
		}
	}

	async function addUser() {
		await UserService.createUserGlobally({
			requestBody: {
				email,
				password,
				super_admin: is_super_admin,
				name,
				company
			}
		})
		sendUserToast(`Added ${email}`)
		dispatch('new')
	}
</script>

<div class="flex flex-row space-x-1">
	<input type="email" on:keyup={handleKeyUp} placeholder="email" bind:value={email} />

	<Toggle class="mx-2" bind:checked={is_super_admin} options={{ right: 'superadmin' }} />
	<input on:keyup={handleKeyUp} type="password" placeholder="password" bind:value={password} />
	<input type="text" on:keyup={handleKeyUp} placeholder="name" bind:value={name} />
	<input type="text" on:keyup={handleKeyUp} placeholder="company" bind:value={company} />

	<Button
		variant="contained"
		color="blue"
		size="sm"
		btnClasses="!ml-4 !w-40"
		on:click={addUser}
		disabled={email == undefined || password == undefined}
	>
		Add
	</Button>
</div>
