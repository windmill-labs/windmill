<script lang="ts">
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice,
		type OnBehalfOfDetails
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import { userStore, workspaceStore } from '$lib/stores'

	interface Props {
		/** Current permissioned_as value from the trigger (e.g., 'u/admin') */
		permissionedAs: string | undefined
		/** Callback when user changes the permissioned_as selection */
		onPermissionedAsChange: (permissionedAs: string | undefined, preserve: boolean) => void
	}

	let { permissionedAs, onPermissionedAsChange }: Props = $props()

	const canPreserve = $derived(
		$userStore?.is_admin || ($userStore?.groups ?? []).includes('wm_deployers')
	)

	let onBehalfOfChoice = $state<OnBehalfOfChoice>(undefined)
	let customPermissionedAs = $state<string | undefined>(undefined)

	function handleSelect(choice: OnBehalfOfChoice, details?: OnBehalfOfDetails) {
		onBehalfOfChoice = choice
		if (choice === 'target') {
			customPermissionedAs = undefined
			onPermissionedAsChange(permissionedAs, true)
		} else if (choice === 'custom' && details) {
			customPermissionedAs = details.permissionedAs
			onPermissionedAsChange(details.permissionedAs, true)
		} else {
			customPermissionedAs = undefined
			onPermissionedAsChange(undefined, false)
		}
	}
</script>

{#if permissionedAs && $workspaceStore}
	<div class="flex items-center gap-2 text-xs text-secondary">
		<span>Permissioned as</span>
		<OnBehalfOfSelector
			targetWorkspace={$workspaceStore}
			targetValue={permissionedAs}
			selected={onBehalfOfChoice}
			onSelect={handleSelect}
			kind="trigger"
			{canPreserve}
			customValue={customPermissionedAs}
			isDeployment={false}
		/>
	</div>
{/if}
