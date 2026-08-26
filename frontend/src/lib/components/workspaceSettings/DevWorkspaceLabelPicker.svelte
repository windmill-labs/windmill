<script lang="ts">
	import Select from '$lib/components/select/Select.svelte'
	import { DEV_WORKSPACE_LABELS, type DevWorkspaceLabelKey } from '$lib/utils/devWorkspaceLabel'

	let {
		value = $bindable(),
		takenLabels
	}: {
		value: DevWorkspaceLabelKey
		/** Labels already held by a dev workspace in the resulting chain; the backend rejects a reuse. */
		takenLabels: Set<string>
	} = $props()

	// Dev workspaces in a chain share their git-sync repositories, so two carrying the same label
	// deploy to the same branch. Offer only what is left rather than a choice the backend rejects.
	let free = $derived(DEV_WORKSPACE_LABELS.filter((l) => !takenLabels.has(l)))
	// Named, not badge-abbreviated: the label picked here is the branch that gets created, so
	// offering `stg` for a `staging` branch would name something that does not exist.
	let items = $derived(free.map((l) => ({ label: l, value: l })))
	$effect(() => {
		if (free.length > 0 && !free.includes(value)) value = free[0]
	})
</script>

<div class="flex items-center gap-2 text-2xs text-secondary">
	<span>Label:</span>
	<div class="w-40">
		<Select {items} bind:value size="sm" />
	</div>
</div>
