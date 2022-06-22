<script lang="ts">
	import { sendUserToast } from '$lib/utils'
	import Switch from './Switch.svelte'

	import type Modal from './Modal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'

	const dispatch = createEventDispatcher()

	let valid = true

	let modal: Modal

	export function openModal(): void {
		modal.openModal()
	}

	let email: string
	let is_admin = false

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key || event.keyCode
		if (key === 13 || key === 'Enter') {
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
	<input on:keyup={handleKeyUp} placeholder="email" bind:value={email} />

	<Switch class="ml-2" bind:checked={is_admin} horizontal={true} label={'admin: '} />
	<button
		class="ml-4 w-40 {valid ? 'default-button' : 'default-button-disabled'}"
		type="button"
		on:click={() => {
			inviteUser()
		}}
		disabled={email == undefined}
	>
		Invite
	</button>
</div>
