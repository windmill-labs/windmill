<script lang="ts">
	import { UserService } from '$lib/gen'
	import { Button } from './common'
	import { sendUserToast } from '$lib/toast'
	import Alert from './common/alert/Alert.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		email: string
		username?: string | undefined
		noPadding?: boolean
	}

	let { email, username = undefined, noPadding = false }: Props = $props()

	let editedEmail: string | undefined = $state(undefined)
	let newEmail = $derived((editedEmail ?? email).trim())
	let loading = $state(false)

	const dispatch = createEventDispatcher()

	async function changeEmail() {
		loading = true
		try {
			await UserService.globalUserChangeEmail({ email, requestBody: { new_email: newEmail } })
			sendUserToast(`Changed email of ${email} to ${newEmail}`)
			dispatch('changed')
		} finally {
			loading = false
		}
	}
</script>

<div class="flex flex-col max-w-2xl {noPadding ? '' : 'p-4'}">
	<span class="text-xs font-semibold text-emphasis mb-1 leading-6">Email</span>
	<TextInput
		inputProps={{
			type: 'email',
			onclick: (e) => {
				e.stopPropagation()
			},
			onkeydown: (e) => {
				e.stopPropagation()
			},
			onkeypress: (e) => {
				e.stopPropagation()
				if (e.key === 'Enter') {
					changeEmail()
				}
			}
		}}
		bind:value={() => editedEmail ?? email, (v) => (editedEmail = String(v))}
	/>

	<Alert title="What is preserved" class="mt-2 mb-2" size="xs">
		The account keeps its instance-wide username{username ? ` (${username})` : ''}, role, workspace
		memberships, drafts and tokens. Audit logs keep the previous email, pending password reset links
		are invalidated, and an SSO user must be able to sign in with the new email.
	</Alert>

	<Button
		variant="default"
		unifiedSize="md"
		btnClasses="mt-2"
		disabled={!newEmail || newEmail === email}
		{loading}
		on:click={() => changeEmail()}
	>
		Update email
	</Button>
</div>
