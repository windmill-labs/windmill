<script lang="ts">
	import RunOnBehalfOfSetting from './RunOnBehalfOfSetting.svelte'
	import { appOnBehalfOfTooltip, deployOnBehalfOfQuestion } from './onBehalfOf'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'

	/** The body of a deploy's question about the user an item runs on behalf of, which the host
	 *  titles with `deployOnBehalfOfQuestion`. */
	interface Props {
		kind: 'flow' | 'script' | 'app'
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
		/** Inside a `ConfirmationModal`: see `OnBehalfOfSelector`. */
		aboveConfirmationModal?: boolean
		onPickerOpenChange?: (open: boolean) => void
	}

	let {
		kind,
		email = $bindable(),
		permissionedAs = $bindable(),
		savedEmail,
		savedPermissionedAs,
		isNew,
		folderDefault,
		targetWorkspace,
		executionMode,
		rawApp = false,
		aboveConfirmationModal = false,
		onPickerOpenChange
	}: Props = $props()

	const operatingUser = useOperatingUser()
	const question = $derived(
		deployOnBehalfOfQuestion(kind, operatingUser.current, {
			email,
			permissionedAs,
			savedEmail,
			savedPermissionedAs,
			isNew,
			folderDefault
		})
	)
</script>

<p>
	{#if question.stops}
		Runs will use the access of whoever starts them, not <strong>{question.savedUser}</strong>.
	{:else if !question.on}
		Runs will use the access of whoever starts them.
	{:else if kind === 'app'}
		The app's runnables use the access of <strong>{question.runsAs ?? 'your user'}</strong> to
		resources and variables.
		{#if executionMode === 'anonymous'}
			Anyone with the secret URL can run them, and runs by visitors who aren't signed in are
			recorded as <code>anonymous</code>.
		{:else}
			Runs are still recorded under the viewer's name, and the code reads their email from
			<code>WM_END_USER_EMAIL</code>.
		{/if}
	{:else}
		Every run uses the access of <strong>{question.runsAs ?? 'your user'}</strong> to resources and
		variables. Who started it is still recorded, and the code reads their email from
		<code>WM_END_USER_EMAIL</code>.
	{/if}
</p>
<div class="flex flex-col gap-1">
	<RunOnBehalfOfSetting
		{kind}
		required={question.required}
		tooltip={kind === 'app' ? appOnBehalfOfTooltip(rawApp) : undefined}
		bind:email
		bind:permissionedAs
		{savedEmail}
		{savedPermissionedAs}
		{targetWorkspace}
		{isNew}
		{folderDefault}
		{aboveConfirmationModal}
		{onPickerOpenChange}
		hideDescription
	/>
	{#if question.on && question.unchanged}
		<!-- pl-9 lines it up under the switch's label, as SettingToggle indents its contents. -->
		<div class={['text-2xs text-secondary', !question.required && 'pl-9']}>
			Same as the deployed version.
		</div>
	{/if}
</div>
