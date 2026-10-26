<script lang="ts">
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { userStore } from '$lib/stores'
	import { AlertTriangle } from 'lucide-svelte'
	import SettingToggle from './SettingToggle.svelte'
	import { canPreserveOnBehalfOf, deployedPermissionedAs, WM_DEPLOYERS_GROUP } from './onBehalfOf'

	interface Props {
		kind: 'flow' | 'script'
		/** Email of the user whose access the runs use; undefined uses whoever starts each run. */
		email: string | undefined
		/** `u/username` of that user. */
		permissionedAs: string | undefined
		/** Sent as `preserve_on_behalf_of`: keep the picked user rather than the deployer. */
		preserve: boolean
		/** The selector's choice. Owned by the host, so it outlives this row's remounts: while it is
		 *  undefined the selector picks the deployed user, which would override a later pick. */
		choice: OnBehalfOfChoice
		/** The deployed identity, which the "target" choice keeps. */
		savedEmail: string | undefined
		savedPermissionedAs: string | undefined
		targetWorkspace: string
		/** Never deployed, so no deployed identity can change. */
		isNew?: boolean
	}

	let {
		kind,
		email = $bindable(),
		permissionedAs = $bindable(),
		preserve = $bindable(),
		choice = $bindable(),
		savedEmail,
		savedPermissionedAs,
		targetWorkspace,
		isNew = false
	}: Props = $props()

	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)
	let canPreserve = $derived(canPreserveOnBehalfOf(actingUser))
	let myPermissionedAs = $derived($userStore?.username ? `u/${$userStore.username}` : undefined)

	let enabled = $derived(!!email)
	let chosenUser = $derived(permissionedAs ?? email ?? 'the chosen user')
	let deployPermissionedAs = $derived(
		enabled ? deployedPermissionedAs(actingUser, permissionedAs) : undefined
	)
	let willChange = $derived(
		!isNew &&
			!!savedPermissionedAs &&
			!!deployPermissionedAs &&
			deployPermissionedAs !== savedPermissionedAs
	)
</script>

{#snippet valueHint()}
	Lets people run this {kind} with resources and variables they can't access themselves.
{/snippet}

<SettingToggle
	setting="on-behalf-of"
	label="Run on behalf of"
	tooltip={`Use it to share a ${kind} that touches secrets: people run it without ever seeing the credentials. Or pick a dedicated user, like billing-bot, so the ${kind} can only reach that user's folders and resources.`}
	documentationLink="https://www.windmill.dev/docs/core_concepts/roles_and_permissions#run-on-behalf-of"
	hint={valueHint}
	checked={enabled}
	onchange={(on) => {
		if (on) {
			email = $userStore?.email
			permissionedAs = myPermissionedAs
		} else {
			email = undefined
			permissionedAs = undefined
			preserve = false
			choice = undefined
		}
	}}
>
	<div class="flex flex-wrap items-center gap-1.5">
		<OnBehalfOfSelector
			{targetWorkspace}
			targetValue={savedEmail}
			selected={choice}
			onSelect={(c, details) => {
				choice = c
				if (c === 'me') {
					email = $userStore?.email
					permissionedAs = myPermissionedAs
					preserve = false
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
			customValue={choice === 'custom' ? email : undefined}
			isDeployment={false}
			readOnlyWithoutPreserve
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
	<div class="text-2xs text-secondary">
		Every run uses the access of {chosenUser}. Who started it is still recorded, and the code reads
		their email from <code>WM_END_USER_EMAIL</code>.
	</div>
</SettingToggle>
