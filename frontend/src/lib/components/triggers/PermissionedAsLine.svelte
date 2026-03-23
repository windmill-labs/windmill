<script lang="ts">
	import { Alert } from '$lib/components/common'
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

	const myPermissionedAs = $derived($userStore?.username ? `u/${$userStore.username}` : undefined)

	let onBehalfOfChoice = $state<OnBehalfOfChoice>(undefined)
	let customPermissionedAs = $state<string | undefined>(undefined)

	// The effective value that will be saved
	const effectivePermissionedAs = $derived.by(() => {
		if (onBehalfOfChoice === 'target') return permissionedAs
		if (onBehalfOfChoice === 'custom' && customPermissionedAs) return customPermissionedAs
		return myPermissionedAs
	})

	const willChange = $derived(
		permissionedAs !== undefined &&
			effectivePermissionedAs !== undefined &&
			permissionedAs !== effectivePermissionedAs
	)

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
	<Alert type={willChange ? 'warning' : 'info'} size="xs" title="Permissioned as">
		<div class="flex items-center gap-2 flex-wrap">
			{#if willChange}
				<span>
					Currently <span class="font-mono font-semibold">{permissionedAs}</span>, will change to
					<span class="font-mono font-semibold">{effectivePermissionedAs}</span>
					on save.
				</span>
			{:else}
				<span class="font-mono font-semibold">{permissionedAs}</span>
			{/if}
			{#if canPreserve}
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
			{/if}
		</div>
	</Alert>
{/if}
