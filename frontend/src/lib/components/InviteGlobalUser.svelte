<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import { UserService } from '$lib/gen'
	import { Button } from './common'
	import Toggle from './Toggle.svelte'
	import { generateRandomString } from '$lib/utils'

	const dispatch = createEventDispatcher()

	let email: string
	let is_super_admin = false
	let password: string = generateRandomString(10)
	let name: string | undefined
	let company: string | undefined

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

<div class="flex flex-row gap-2 mb-2 items-end">
	<label class="block shrink min-w-0">
		<span class="text-gray-700 text-sm">Email</span>
		<input type="email" placeholder="email" bind:value={email} />
	</label>
	<label class="block shrink min-w-0">
		<span class="text-gray-700 text-sm">Password</span>
		<input bind:value={password} />
	</label>

	<Toggle class="mx-2" bind:checked={is_super_admin} options={{ right: 'Superadmin' }} />
	<div class="flex flex-row-reverse grow">
		<div class="flex">
			<Button
				variant="contained"
				color="dark"
				size="sm"
				on:click={addUser}
				disabled={email == undefined || password == undefined}
			>
				Add user to instance
			</Button>
		</div>
	</div>
</div>
<div class="flex gap-2 items-end">
	<div>
		<input type="text" placeholder="name (optional)" bind:value={name} />
	</div>
	<div>
		<input type="text" placeholder="company (optional)" bind:value={company} />
	</div>
	<div class="text-xs text-gray-600 grow text-right"> Email will be sent if SMTP configured </div>
</div>
