<script lang="ts">
	import type { AppWithLastVersion } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'
	import { useEditRights, useRoleCapabilities } from '$lib/operatorWriteRights'
	import AppAccessSection from './AppAccessSection.svelte'
	import {
		accessModeOf,
		modeChangeRefusal,
		updateDeployedAppAccess,
		type AppAccessMode
	} from './appAccess'
	import { canPreserveOnBehalfOf } from './onBehalfOf'

	interface Props {
		workspace: string
		/** The deployed row, rewritten here after each change. */
		app: AppWithLastVersion
		/** Re-reads the row, after a refused change. */
		reload: () => Promise<void>
		/** Called after a write to the app row, with the row as it now is. */
		onAppChanged?: (row: AppWithLastVersion) => void
	}

	let { workspace, app = $bindable(), reload, onAppChanged }: Props = $props()

	const operatingUser = useOperatingUser()
	const editRights = useEditRights()
	const roleCaps = useRoleCapabilities()

	let busy = $state(false)
	let name = $derived(app.path.split('/').pop() ?? app.path)
	let runAs = $derived(app.policy.on_behalf_of ?? app.policy.on_behalf_of_email)
	// Who can open the app is part of its policy, which only someone who can edit the app may
	// rewrite.
	let canWrite = $derived(
		editRights.canEditItem(app.raw_app ? 'raw_app' : 'app', app.path, app.extra_perms)
	)
	let refusal = $derived(
		modeChangeRefusal(app.policy, {
			username: operatingUser.current?.username,
			canPreserveRunAs: canPreserveOnBehalfOf(operatingUser.current),
			composeOnly: $roleCaps.apps === 'compose'
		})
	)

	const ACCESS_MODE_LABELS: Record<AppAccessMode, string> = {
		publisher: 'Members',
		guest: 'Guests',
		anonymous: 'Public'
	}

	async function change(
		change: { execution_mode: AppAccessMode } | { custom_path: string | null }
	) {
		busy = true
		try {
			app = await updateDeployedAppAccess(workspace, app.path, change)
			onAppChanged?.(app)
			sendUserToast(
				'execution_mode' in change
					? `Who can open ${name} changed to ${ACCESS_MODE_LABELS[change.execution_mode]}`
					: change.custom_path
						? `Custom URL of ${name} set to ${change.custom_path}`
						: `Custom URL of ${name} removed`
			)
		} catch (err) {
			sendUserToast(`Could not change the app's access: ${err?.body ?? err}`, true)
			// Re-read so the controls show what is deployed, not the choice that was refused.
			await reload()
		} finally {
			busy = false
		}
	}
</script>

{#if !canWrite}
	<span class="text-xs text-secondary">
		Only people who can edit this app can change who can open it.
	</span>
{:else if refusal}
	<span class="text-xs text-secondary">
		{#if refusal === 'unsandboxed'}
			This app is not sandboxed yet. Deploy it so it is sandboxed, then change who can open it.
		{:else if refusal === 'builder_run_as'}
			This app runs on behalf of {runAs}. Pick yourself in the editor's Settings › Permissions and
			deploy it, then change who can open it. An admin can change it without taking the app over.
		{:else}
			This app runs on behalf of {runAs}. Deploy it so it runs on behalf of you, then change who can
			open it. An admin or a member of <code>wm_deployers</code> can change it without taking the app
			over.
		{/if}
	</span>
{/if}
{#key app.path}
	<AppAccessSection
		{workspace}
		appPath={app.path}
		rawApp={app.raw_app}
		mode={accessModeOf(app.policy)}
		onModeChange={(m) => change({ execution_mode: m })}
		deployed
		customPath={app.custom_path ?? undefined}
		onCustomPathChange={(p) => change({ custom_path: p })}
		disabled={busy || !canWrite || !!refusal}
	/>
{/key}
