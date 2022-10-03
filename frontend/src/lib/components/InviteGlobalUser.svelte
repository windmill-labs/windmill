<script lang="ts">
	import { sendUserToast } from '$lib/utils'
	import Switch from './Switch.svelte'
	import type Modal from './Modal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { UserService } from '$lib/gen'
	import { Button } from './common'

	const dispatch = createEventDispatcher()

	let modal: Modal

	export function openModal(): void {
		modal.openModal()
	}

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
		sendUserToast(`Successfully added ${email}. Welcome to them!`)
		dispatch('new')
	}
</script>

<div class="flex flex-row">
	<input on:keyup={handleKeyUp} placeholder="email" bind:value={email} />

	<Switch class="ml-2" bind:checked={is_super_admin} horizontal={true} label={'admin: '} />
	<input on:keyup={handleKeyUp} type="password" placeholder="password" bind:value={password} />
	<input on:keyup={handleKeyUp} placeholder="name" bind:value={name} />
	<input on:keyup={handleKeyUp} placeholder="company" bind:value={company} />

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
