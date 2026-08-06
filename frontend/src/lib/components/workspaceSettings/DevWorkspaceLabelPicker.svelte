<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import {
		DEV_WORKSPACE_LABELS,
		devBadgeText,
		devLabelError,
		type DevWorkspaceLabelKey
	} from '$lib/utils/devWorkspaceLabel'

	let {
		value = $bindable(),
		takenLabels
	}: {
		value: DevWorkspaceLabelKey
		/** Labels already held by a dev workspace in the resulting chain; the backend rejects a reuse. */
		takenLabels: Set<string>
	} = $props()

	let free = $derived(DEV_WORKSPACE_LABELS.filter((l) => !takenLabels.has(l)))
	// A custom label is the only way through once both offered labels are taken, so the input opens
	// itself there. Otherwise it stays behind the link: naming an environment anything other than
	// dev/staging is rare, and the common path stays a single toggle.
	let custom = $state(false)
	let editing = $derived(custom || free.length === 0)
	// Always shown while editing, never gated on the user having typed: the value the input opens
	// with is the taken one whenever the chain forced it open, and the parent disables its submit
	// button off the same two checks — a silent disabled button would leave no way to find out why.
	let error = $derived.by(() => {
		if (!editing) return undefined
		const invalid = devLabelError(value)
		if (invalid) return invalid
		return takenLabels.has(value.trim())
			? `'${value.trim()}' is already taken by a dev workspace in this chain, which would deploy to the same branch. Name this environment something else.`
			: undefined
	})
	$effect(() => {
		if (!editing && !free.includes(value)) value = free[0]
	})
</script>

<div class="flex flex-col gap-1">
	<div class="flex items-center gap-2 text-2xs text-secondary">
		<span>Label:</span>
		{#if editing}
			<div class="w-40">
				<TextInput
					bind:value
					size="xs"
					error={!!error}
					inputProps={{
						placeholder: 'e.g. uat',
						spellcheck: false,
						'aria-label': 'Environment label'
					}}
				/>
			</div>
			{#if free.length > 0}
				<Button variant="subtle" unifiedSize="2xs" onclick={() => (custom = false)}>
					Use {free.join(' or ')}
				</Button>
			{/if}
		{:else}
			<Badge color="indigo" small>{devBadgeText(value)}</Badge>
			{#if free.length > 1}
				<Button
					variant="subtle"
					unifiedSize="2xs"
					onclick={() => (value = free[(free.indexOf(value) + 1) % free.length])}
				>
					Change to {free.find((l) => l !== value)}
				</Button>
			{/if}
			<Button variant="subtle" unifiedSize="2xs" onclick={() => (custom = true)}>Custom</Button>
		{/if}
	</div>
	{#if error}
		<span class="text-2xs text-red-600 dark:text-red-400">{error}</span>
	{:else if !editing && free.length === 1}
		<span class="text-2xs text-secondary">
			The other label is already taken by a dev workspace in this chain, which would deploy to the
			same branch.
		</span>
	{/if}
</div>
