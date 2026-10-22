<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import { userStore } from '$lib/stores'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import SettingToggle from './SettingToggle.svelte'
	import { canPreserveOnBehalfOf, deployedPermissionedAs, WM_DEPLOYERS_GROUP } from './onBehalfOf'
	const DOCS_LINK =
		'https://www.windmill.dev/docs/core_concepts/roles_and_permissions#run-on-behalf-of'

	interface Props {
		kind: 'flow' | 'script' | 'app'
		/** Email of the user whose access the runs use; undefined uses whoever starts each run. */
		email: string | undefined
		/** `u/username` of that user. */
		permissionedAs: string | undefined
		/** Sent as `preserve_on_behalf_of` on deploy. Undefined until a user is picked here, which
		 *  the host reads as "keep the deployed identity". */
		preserve: boolean | undefined
		/** The deployed identity, which the "target" choice keeps. */
		savedEmail: string | undefined
		savedPermissionedAs: string | undefined
		targetWorkspace: string
		/** Apps always use a chosen user's access: no toggle, only the user choice. */
		required?: boolean
		/** Never deployed, so no deployed identity can change. */
		isNew?: boolean
		tooltip?: string
		/** Inside a `ConfirmationModal`: see `OnBehalfOfSelector`. */
		aboveConfirmationModal?: boolean
		onPickerOpenChange?: (open: boolean) => void
		/** The host explains the setting itself. */
		hideDescription?: boolean
	}

	let {
		kind,
		email = $bindable(),
		permissionedAs = $bindable(),
		preserve = $bindable(),
		savedEmail,
		savedPermissionedAs,
		targetWorkspace,
		required = false,
		isNew = false,
		tooltip,
		aboveConfirmationModal = false,
		onPickerOpenChange,
		hideDescription = false
	}: Props = $props()

	// The label keeps the docs' term; the description says what it gives, access to resources,
	// and that the person who starts a run stays recorded and readable from WM_END_USER_EMAIL.
	let shownTooltip = $derived(
		tooltip ??
			`Use it to share a ${kind} that touches secrets: people run it without ever seeing the credentials. Or pick a dedicated user, like billing-bot, so the ${kind} can only reach that user's folders and resources.`
	)

	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)
	let canPreserve = $derived(canPreserveOnBehalfOf(actingUser))
	// Usernames are per-workspace, so "me" is the acting user in the target workspace.
	let myPermissionedAs = $derived(actingUser?.username ? `u/${actingUser.username}` : undefined)

	// Read from the bound values, not kept here: this row remounts each time its tab or dialog
	// opens, and a choice that restarts empty lets the selector reselect the deployed user over
	// the one picked.
	let choice: OnBehalfOfChoice = $derived.by(() => {
		if (preserve === false) return 'me'
		const keepsSaved = savedPermissionedAs
			? (permissionedAs ?? savedPermissionedAs) === savedPermissionedAs
			: !!savedEmail && (email ?? savedEmail) === savedEmail
		if (keepsSaved) return 'target'
		const isMe = permissionedAs
			? permissionedAs === myPermissionedAs
			: !email || email === $userStore?.email
		return isMe ? 'me' : 'custom'
	})

	let enabled = $derived(required || !!email)
	let chosenUser = $derived(permissionedAs ?? email ?? 'the chosen user')
	// An unset `preserve` keeps the deployed identity, as the hosts send it.
	let deployPermissionedAs = $derived(
		enabled
			? deployedPermissionedAs(
					actingUser,
					permissionedAs,
					preserve ?? !!(savedPermissionedAs || savedEmail)
				)
			: undefined
	)
	let willChange = $derived(
		!isNew &&
			!!savedPermissionedAs &&
			!!deployPermissionedAs &&
			deployPermissionedAs !== savedPermissionedAs
	)

	function setMe() {
		email = $userStore?.email
		permissionedAs = myPermissionedAs
		preserve = false
	}
</script>

{#snippet picker()}
	<div class="flex flex-wrap items-center gap-1.5">
		<OnBehalfOfSelector
			{targetWorkspace}
			targetValue={savedEmail}
			selected={choice}
			onSelect={(c, details) => {
				if (c === 'me') {
					setMe()
				} else if (c === 'target') {
					// Keep the saved pair. An item that has no recorded principal yet sends the
					// email alone and the backend derives one from it.
					email = savedEmail
					permissionedAs = savedPermissionedAs
					preserve = true
				} else if (c === 'custom' && details) {
					email = details.email
					permissionedAs = details.permissionedAs
					preserve = true
				}
			}}
			{kind}
			{canPreserve}
			customValue={choice === 'custom' ? (permissionedAs ?? email) : undefined}
			isDeployment={false}
			{myPermissionedAs}
			readOnlyWithoutPreserve
			{aboveConfirmationModal}
			{onPickerOpenChange}
		/>
		{#if willChange}
			<AlertTriangle size={14} class="text-yellow-500" />
			<span class="text-2xs text-yellow-600 dark:text-yellow-400"
				>Changes from <strong>{savedPermissionedAs}</strong> on deploy</span
			>
		{/if}
	</div>
	{#if !canPreserve}
		<div class="text-2xs text-secondary">
			Only workspace admins and members of {WM_DEPLOYERS_GROUP} can pick another user.
		</div>
	{/if}
	{#if !hideDescription}
		<div class="text-2xs text-secondary">
			{#if required}
				Apps always run on behalf of someone: the people who open an app usually can't access its
				scripts and resources, and guest or public viewers have no account at all. Viewers can start
				only the runnables built into the app, and each one still gets the viewer's email in
				<code>WM_END_USER_EMAIL</code>.
			{:else}
				Every run uses the access of {chosenUser}. Who started it is still recorded, and the code
				reads their email from <code>WM_END_USER_EMAIL</code>.
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet valueHint()}
	Lets people run this {kind} with resources and variables they can't access themselves.
{/snippet}

{#if required}
	<div data-setting="on-behalf-of">
		<Label label="Run on behalf of">
			{#snippet header()}
				<!-- The tooltip also renders an empty trigger span; one wrapper keeps the label gap single. -->
				<span class="inline-flex">
					<Tooltip documentationLink={DOCS_LINK}>
						{#snippet text()}{shownTooltip}{/snippet}
					</Tooltip>
				</span>
			{/snippet}
			{@render picker()}
		</Label>
	</div>
{:else}
	<SettingToggle
		setting="on-behalf-of"
		label="Run on behalf of"
		tooltip={shownTooltip}
		documentationLink={DOCS_LINK}
		hint={valueHint}
		checked={enabled}
		onchange={(on) => {
			if (on) {
				setMe()
			} else {
				email = undefined
				permissionedAs = undefined
				preserve = false
			}
		}}
	>
		{@render picker()}
	</SettingToggle>
{/if}
