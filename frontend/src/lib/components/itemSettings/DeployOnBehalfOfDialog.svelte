<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import DeployOnBehalfOfFields from './DeployOnBehalfOfFields.svelte'
	import { deployOnBehalfOfQuestion } from './onBehalfOf'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import {
		createPendingServiceAccount,
		getPendingServiceAccount
	} from '$lib/components/access/serviceAccounts.svelte'
	import type { NeedsSource } from '$lib/components/access/detectNeeds'
	import { sendUserToast } from '$lib/toast'

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
		/** While it loads the dialog can't name who the deploy runs as, so it can't be confirmed. */
		folderDefaultLoading?: boolean
		targetWorkspace: string
		/** Apps: who can open the app, which decides whether viewers are recorded at all. */
		executionMode?: string
		rawApp?: boolean
		/** The item's path and its editor's copy: what a service account made here is built from. */
		itemPath?: string
		needsSource?: () => NeedsSource | undefined
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
		folderDefaultLoading = false,
		targetWorkspace,
		executionMode,
		rawApp = false,
		itemPath,
		needsSource,
		onConfirmed
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
	let pickerOpen = $state(false)
	let overlayOpen = $state(false)
	let deploying = $state(false)

	async function confirm() {
		const runsAs = question.runsAs
		if (getPendingServiceAccount(targetWorkspace, runsAs)) {
			deploying = true
			try {
				const created = await createPendingServiceAccount(targetWorkspace, runsAs)
				if (created) email = created
			} catch (e) {
				sendUserToast(`Could not create ${runsAs}: ${e?.body ?? e?.message ?? e}`, true)
				return
			} finally {
				deploying = false
			}
		}
		open = false
		onConfirmed()
	}
</script>

<ConfirmationModal
	{open}
	type="info"
	title={question.title}
	confirmationText="Deploy"
	confirmDisabled={folderDefaultLoading}
	loading={deploying}
	keyListen={!pickerOpen && !overlayOpen}
	alwaysPortal
	onConfirmed={confirm}
	onCanceled={() => (open = false)}
>
	<div class="flex flex-col gap-4">
		<DeployOnBehalfOfFields
			{kind}
			bind:email
			bind:permissionedAs
			{savedEmail}
			{savedPermissionedAs}
			{isNew}
			{folderDefault}
			{targetWorkspace}
			{executionMode}
			{rawApp}
			aboveConfirmationModal
			onPickerOpenChange={(o) => (pickerOpen = o)}
			{itemPath}
			{needsSource}
			onOverlayChange={(o) => (overlayOpen = o)}
		/>
		<TextInput bind:value={message} inputProps={{ placeholder: 'Deployment message' }} />
	</div>
</ConfirmationModal>
