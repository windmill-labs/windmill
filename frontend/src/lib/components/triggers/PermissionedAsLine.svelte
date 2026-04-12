<script lang="ts">
	import OnBehalfOfSelector, {
		type OnBehalfOfChoice,
		type OnBehalfOfDetails
	} from '$lib/components/OnBehalfOfSelector.svelte'
	import { useFolderDefaultPermissionedAs } from '$lib/components/useFolderDefaultPermissionedAs.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { AlertTriangle } from 'lucide-svelte'

	interface Props {
		/** Current permissioned_as value from the trigger (e.g., 'u/admin') */
		permissionedAs: string | undefined
		/** Callback when user changes the permissioned_as selection */
		onPermissionedAsChange: (permissionedAs: string | undefined, preserve: boolean) => void
		/**
		 * Item path (e.g. `f/prod/my_trigger`). When provided and the user is an
		 * admin/wm_deployers member, the component fetches the parent folder's
		 * `default_permissioned_as` rules and preselects the matching default.
		 */
		path?: string | undefined
	}

	let { permissionedAs, onPermissionedAsChange, path = undefined }: Props = $props()

	const canPreserve = $derived(
		$userStore?.is_admin || ($userStore?.groups ?? []).includes('wm_deployers')
	)

	const myPermissionedAs = $derived($userStore?.username ? `u/${$userStore.username}` : undefined)

	const folderDefault = useFolderDefaultPermissionedAs(() => path)

	let onBehalfOfChoice = $state<OnBehalfOfChoice>(undefined)
	let customPermissionedAs = $state<string | undefined>(undefined)

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

	const shouldRender = $derived(
		!!$workspaceStore &&
			(permissionedAs !== undefined || (canPreserve && folderDefault.value !== undefined))
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

{#if shouldRender && $workspaceStore}
	<div class="flex items-center gap-1.5 text-2xs text-tertiary mb-4">
		<span>Permissioned as</span>
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
				folderDefault={folderDefault.value}
			/>
			{#if willChange}
				<AlertTriangle class="w-3.5 h-3.5 text-yellow-500" />
				<span class="text-yellow-600 dark:text-yellow-400"
					>will change from <strong>{permissionedAs}</strong> on save</span
				>
			{/if}
		{:else if permissionedAs}
			<strong class="text-secondary">{permissionedAs}</strong>
			{#if willChange}
				<AlertTriangle class="w-3.5 h-3.5 text-yellow-500" />
				<span class="text-yellow-600 dark:text-yellow-400"
					>will change to <strong>{effectivePermissionedAs}</strong> on save</span
				>
			{/if}
		{/if}
	</div>
{/if}
