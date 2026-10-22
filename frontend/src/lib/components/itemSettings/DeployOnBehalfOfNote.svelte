<script lang="ts">
	import { AlertTriangle } from 'lucide-svelte'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { deployedPermissionedAs } from './onBehalfOf'

	interface Props {
		/** Apps always run on behalf of a user; flows and scripts only when the setting is on. */
		enabled: boolean
		/** `u/username` in the working copy. */
		permissionedAs: string | undefined
		/** `u/username` of the deployed item, undefined before the first deploy. */
		savedPermissionedAs: string | undefined
		/** The `preserve_on_behalf_of` flag the deploy will send. */
		preserve: boolean | undefined
	}

	let { enabled, permissionedAs, savedPermissionedAs, preserve }: Props = $props()

	const operatingUser = useOperatingUser()
	const me = $derived(
		operatingUser.current?.username ? `u/${operatingUser.current.username}` : undefined
	)
	const deployed = $derived(deployedPermissionedAs(operatingUser.current, permissionedAs, preserve))
	const changes = $derived(!!savedPermissionedAs && !!deployed && savedPermissionedAs !== deployed)
</script>

{#if enabled && deployed}
	{@const who = deployed === me ? `${deployed} (you)` : deployed}
	{#if changes}
		<div class="flex items-start gap-1.5 text-2xs text-yellow-700 dark:text-yellow-400">
			<AlertTriangle size={12} class="mt-0.5 shrink-0" />
			<span
				>Will run on behalf of <strong>{who}</strong> instead of
				<strong>{savedPermissionedAs}</strong>. Change it in Settings › Permissions.</span
			>
		</div>
	{:else}
		<div class="text-2xs text-secondary">
			Runs on behalf of <strong class="text-primary">{who}</strong>. Change it in Settings ›
			Permissions.
		</div>
	{/if}
{/if}
