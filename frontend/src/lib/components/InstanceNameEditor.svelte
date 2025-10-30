<script lang="ts">
	import { stopPropagation, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { Pencil } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Button from './common/button/Button.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import ChangeInstanceUsernameInner from './ChangeInstanceUsernameInner.svelte'
	import { UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		value: string | undefined
		email: string
		username?: string | undefined
		automateUsernameCreation?: boolean
		login_type: string
	}

	let {
		value = $bindable(),
		email,
		username = undefined,
		automateUsernameCreation = false,
		login_type = $bindable()
	}: Props = $props()

	let password: string = $state('')

	const dispatch = createEventDispatcher()

	function saveName() {
		dispatch('save', value)
	}
	async function savePassword() {
		if (password.length < 5) {
			sendUserToast('Password must be at least 5 characters long', true)
			return
		}
		await UserService.setPasswordForUser({ user: email, requestBody: { password } })
		sendUserToast(`Password updated for ${email}`)
	}

	async function saveLoginType() {
		await UserService.setLoginTypeForUser({ user: email, requestBody: { login_type } })
		sendUserToast(`Login type updated for ${email}`)
		dispatch('refresh')
	}
</script>

<Popover
	floatingConfig={{
		strategy: 'fixed',
		placement: 'left-end',
		middleware: [offset(8), flip(), shift()]
	}}
	closeButton
>
	{#snippet trigger()}
		<Button nonCaptureEvent={true} size="xs" color="light" endIcon={{ icon: Pencil }}>Edit</Button>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col gap-8 max-w-sm p-4">
			{#if automateUsernameCreation && username}
				<ChangeInstanceUsernameInner {email} {username} on:renamed noPadding />
			{/if}
			<label class="block text-primary">
				<div class="pb-1 text-xs font-semibold text-emphasis">Name</div>
				<div class="flex w-full">
					<TextInput
						inputProps={{
							onclick: (e) => {
								e.stopPropagation()
							},
							onkeydown: (e) => {
								e.stopPropagation()
							},
							onkeypress: ({ key }) => {
								if (key === 'Enter') {
									saveName()
								}
							}
						}}
						bind:value
					/>
				</div>
				<Button
					unifiedSize="md"
					variant="default"
					buttonType="button"
					btnClasses="mt-2 "
					aria-label="Save ID"
					onclick={() => {
						saveName()
					}}
				>
					Update name
				</Button>
			</label>
			<label class="block text-primary">
				<div class="pb-1 text-xs font-semibold text-emphasis">Password</div>
				<div class="flex w-full">
					<input
						type="password"
						bind:value={password}
						class="!w-auto grow"
						onclick={stopPropagation(() => {})}
						onkeydown={stopPropagation(bubble('keydown'))}
						onkeypress={stopPropagation(({ key }) => {
							if (key === 'Enter') {
								savePassword()
							}
						})}
					/>
				</div>
				<Button
					unifiedSize="md"
					variant="default"
					buttonType="button"
					btnClasses="mt-2 "
					aria-label="Save ID"
					on:click={() => {
						savePassword()
					}}
				>
					Update password
				</Button>
			</label>
			<label class="block text-primary">
				<div class="mb-1 text-xs font-semibold text-emphasis">Login type</div>

				<div class="flex w-full">
					<input
						type="text"
						bind:value={login_type}
						class="!w-auto grow"
						onclick={stopPropagation(() => {})}
						onkeydown={stopPropagation(bubble('keydown'))}
						onkeypress={stopPropagation(({ key }) => {
							if (key === 'Enter') {
								saveLoginType()
							}
						})}
					/>
				</div>
				<div class="text-2xs text-secondary mb-1">
					Must match exact SSO name, "password" or "saml". Examples: password, google, saml,
					microsoft
				</div>
				<Button
					unifiedSize="md"
					variant="default"
					buttonType="button"
					btnClasses="mt-2 "
					aria-label="Save login type"
					on:click={() => {
						saveLoginType()
					}}
				>
					Update login type
				</Button>
			</label>
		</div>
	{/snippet}
</Popover>
