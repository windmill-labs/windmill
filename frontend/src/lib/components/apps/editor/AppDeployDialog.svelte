<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RunOnBehalfOfSetting from '$lib/components/itemSettings/RunOnBehalfOfSetting.svelte'
	import { appOnBehalfOfTooltip } from '$lib/components/itemSettings/onBehalfOf'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import type { Policy } from '$lib/gen'

	interface Props {
		open: boolean
		/** The deployment message, carried over from the Deploy popover. */
		message: string
		/** The editor's own policy: the picker writes the chosen user into it. */
		policy: Policy
		preserveOnBehalfOf: boolean | undefined
		/** The deployed policy, undefined before the first deploy. */
		savedPolicy: Policy | undefined
		/** `u/username` the deploy will run the app on behalf of. */
		runsAs: string | undefined
		targetWorkspace: string
		rawApp?: boolean
		onConfirmed: () => void
	}

	let {
		open = $bindable(),
		message = $bindable(),
		policy,
		preserveOnBehalfOf = $bindable(),
		savedPolicy,
		runsAs,
		targetWorkspace,
		rawApp = false,
		onConfirmed
	}: Props = $props()

	const operatingUser = useOperatingUser()
	const who = $derived(
		!runsAs ? 'you' : runsAs === `u/${operatingUser.current?.username}` ? `${runsAs} (you)` : runsAs
	)
	let pickerOpen = $state(false)
</script>

<ConfirmationModal
	{open}
	type="info"
	title="Run this app on behalf of {who}?"
	confirmationText="Deploy"
	keyListen={!pickerOpen}
	alwaysPortal
	onConfirmed={() => {
		open = false
		onConfirmed()
	}}
	onCanceled={() => (open = false)}
>
	<div class="flex flex-col gap-4">
		<p>
			The app's runnables use the access of <strong>{runsAs ?? 'your user'}</strong> to resources
			and variables.
			{#if policy.execution_mode === 'anonymous'}
				Anyone with the secret URL can run them, and runs by visitors who aren't signed in are
				recorded as <code>anonymous</code>.
			{:else}
				Runs are still recorded under the viewer's name, and the code reads their email from
				<code>WM_END_USER_EMAIL</code>.
			{/if}
		</p>
		<RunOnBehalfOfSetting
			kind="app"
			required
			tooltip={appOnBehalfOfTooltip(rawApp)}
			bind:email={policy.on_behalf_of_email}
			bind:permissionedAs={policy.on_behalf_of}
			bind:preserve={preserveOnBehalfOf}
			savedEmail={savedPolicy?.on_behalf_of_email}
			savedPermissionedAs={savedPolicy?.on_behalf_of}
			{targetWorkspace}
			isNew={!savedPolicy}
			aboveConfirmationModal
			onPickerOpenChange={(o) => (pickerOpen = o)}
			hideDescription
		/>
		<TextInput bind:value={message} inputProps={{ placeholder: 'Deployment message' }} />
	</div>
</ConfirmationModal>
