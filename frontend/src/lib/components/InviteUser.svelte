<script lang="ts">
	import { sendUserToast } from '$lib/utils'
	import type Modal from './Modal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { Button } from './common'
	import Toggle from '$lib/components/Toggle.svelte'

	const dispatch = createEventDispatcher()

	let modal: Modal

	export function openModal(): void {
		modal.openModal()
	}

	let email: string
	let is_admin = false

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			inviteUser()
		}
	}

	async function inviteUser() {
		await WorkspaceService.inviteUser({
			workspace: $workspaceStore!,
			requestBody: {
				email,
				is_admin
			}
		})
		sendUserToast(`Successfully invited ${email}. Welcome to them!`)
		dispatch('new')
	}
</script>

<div class="flex flex-row">
	<input on:keyup={handleKeyUp} placeholder="email" bind:value={email} class="mr-4" />

	<Toggle bind:checked={is_admin} options={{ right: 'admin' }} />
	<Button
		variant="contained"
		color="blue"
		size="sm"
		btnClasses="!ml-8"
		on:click={inviteUser}
		disabled={email === undefined}
	>
		Invite
	</Button>
</div>
