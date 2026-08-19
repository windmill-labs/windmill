<script lang="ts">
	import { UserService } from '$lib/gen'
	import { Button } from './common'
	import { sendUserToast } from '$lib/toast'
	import Alert from './common/alert/Alert.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { createEventDispatcher } from 'svelte'
	import { userStore } from '$lib/stores'

	interface Props {
		email: string
		username?: string | undefined
		noPadding?: boolean
	}

	let { email, username = undefined, noPadding = false }: Props = $props()

	let editedEmail: string | undefined = $state(undefined)
	// Lowercased like the backend normalizes it, so a capitalization-only edit is not offered as a
	// change the backend would then reject.
	let newEmail = $derived((editedEmail ?? email).trim().toLowerCase())
	let isSelf = $derived(email === $userStore?.email)
	let changed = $derived(!!newEmail && newEmail !== email.toLowerCase())
	let loading = $state(false)

	const dispatch = createEventDispatcher()

	async function changeEmail() {
		if (!changed || loading) {
			return
		}
		loading = true
		try {
			await UserService.globalUserChangeEmail({ email, requestBody: { new_email: newEmail } })
			sendUserToast(`Changed email of ${email} to ${newEmail}`)
			dispatch('changed')
		} catch (err) {
			sendUserToast(err.body ?? err.message ?? 'Failed to change email', true)
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
			disabled: isSelf,
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

	{#if isSelf}
		<div class="text-2xs text-tertiary mt-1">
			You cannot change your own email — ask another superadmin.
		</div>
	{/if}

	{#if changed && !isSelf}
		<Alert type="warning" title="Last resort operation" class="mt-2 mb-2" size="xs">
			Changing the email of an existing account is a last resort. Prefer it only when the address
			itself has to change and the account must be kept.
			<br />
			<br />
			The account keeps its instance-wide username{username ? ` (${username})` : ''}, role,
			workspace memberships, drafts and tokens. But past runs and audit logs keep the previous
			email, pending password reset links stop working, and the account inherits any instance group
			membership or workspace invite already addressed to the new email — including one that grants
			a role.
			<br />
			<br />
			If this user signs in through SSO or is managed by SCIM, update the identity provider first: otherwise
			their next login recreates the previous email as a separate, empty account.
		</Alert>
	{/if}

	<Button
		variant="default"
		unifiedSize="md"
		btnClasses="mt-2"
		disabled={!changed || isSelf}
		{loading}
		on:click={() => changeEmail()}
	>
		Update email
	</Button>
</div>
