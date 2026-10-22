<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RunOnBehalfOfSetting from './RunOnBehalfOfSetting.svelte'
	import { appOnBehalfOfTooltip, deployedPermissionedAs } from './onBehalfOf'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'

	interface Props {
		open: boolean
		kind: 'flow' | 'script' | 'app'
		/** The deployment message, carried over from the Deploy popover. */
		message: string
		/** The item's run-as user, which the picker writes; unset runs as whoever starts each run. */
		email: string | undefined
		permissionedAs: string | undefined
		/** The deployed run-as user, unset before the first deploy or when it ran as its caller. */
		savedEmail: string | undefined
		savedPermissionedAs: string | undefined
		isNew: boolean
		/** The folder's default for a new item's path, which the deploy takes when no user is held. */
		folderDefault?: string
		targetWorkspace: string
		/** Apps: who can open the app, which decides whether viewers are recorded at all. */
		executionMode?: string
		rawApp?: boolean
		onConfirmed: () => void
	}

	let {
		open = $bindable(),
		kind,
		message = $bindable(),
		email = $bindable(),
		permissionedAs = $bindable(),
		savedEmail,
		savedPermissionedAs,
		isNew,
		folderDefault,
		targetWorkspace,
		executionMode,
		rawApp = false,
		onConfirmed
	}: Props = $props()

	const operatingUser = useOperatingUser()
	const me = $derived(
		operatingUser.current?.username ? `u/${operatingUser.current.username}` : undefined
	)
	const savedUser = $derived(savedPermissionedAs ?? savedEmail)
	// A flow or script whose deployed version ran on behalf of someone, turned off by this deploy.
	const stops = $derived(
		kind !== 'app' && !email && !permissionedAs && !folderDefault && !!savedUser
	)
	const runsAs = $derived(
		deployedPermissionedAs(operatingUser.current, permissionedAs, folderDefault)
	)
	const who = $derived(!runsAs ? 'you' : runsAs === me ? `${runsAs} (you)` : runsAs)
	const unchanged = $derived(!isNew && !!savedPermissionedAs && runsAs === savedPermissionedAs)
	let pickerOpen = $state(false)
</script>

<ConfirmationModal
	{open}
	type="info"
	title={stops
		? `Stop running this ${kind} on behalf of ${savedUser}?`
		: `Run this ${kind} on behalf of ${who}?`}
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
			{#if stops}
				Runs will use the access of whoever starts them, not <strong>{savedUser}</strong>.
			{:else if kind === 'app'}
				The app's runnables use the access of <strong>{runsAs ?? 'your user'}</strong> to resources
				and variables.
				{#if executionMode === 'anonymous'}
					Anyone with the secret URL can run them, and runs by visitors who aren't signed in are
					recorded as <code>anonymous</code>.
				{:else}
					Runs are still recorded under the viewer's name, and the code reads their email from
					<code>WM_END_USER_EMAIL</code>.
				{/if}
			{:else}
				Every run uses the access of <strong>{runsAs ?? 'your user'}</strong> to resources and
				variables. Who started it is still recorded, and the code reads their email from
				<code>WM_END_USER_EMAIL</code>.
			{/if}
		</p>
		{#if !stops}
			<div class="flex flex-col gap-1">
				<RunOnBehalfOfSetting
					{kind}
					required
					tooltip={kind === 'app' ? appOnBehalfOfTooltip(rawApp) : undefined}
					bind:email
					bind:permissionedAs
					{savedEmail}
					{savedPermissionedAs}
					{targetWorkspace}
					{isNew}
					{folderDefault}
					aboveConfirmationModal
					onPickerOpenChange={(o) => (pickerOpen = o)}
					hideDescription
				/>
				{#if unchanged}
					<div class="text-2xs text-secondary">Same as the deployed version.</div>
				{/if}
			</div>
		{/if}
		<TextInput bind:value={message} inputProps={{ placeholder: 'Deployment message' }} />
	</div>
</ConfirmationModal>
