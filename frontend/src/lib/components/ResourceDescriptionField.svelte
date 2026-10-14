<!-- The description field of a resource, wherever one is created or edited: the
	resource form and the save-as-reusable-agent drawer. One component, for the same
	reason as ResourcePathHint next to it — two copies drift. -->
<script lang="ts">
	import { Pen } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import GfmMarkdown from './GfmMarkdown.svelte'
	import Required from './Required.svelte'
	import TextInput from './text_input/TextInput.svelte'

	interface Props {
		description: string
		label?: string
		placeholder?: string
		canWrite?: boolean
	}

	let {
		description = $bindable(),
		label = 'Resource description',
		placeholder = 'Describe what this resource is for',
		canWrite = true
	}: Props = $props()

	let editing = $state(false)
</script>

<div class="flex flex-col gap-1">
	<h4 class="inline-flex items-center gap-2 text-xs text-emphasis font-semibold"
		>{label}
		<Required required={false} />
		{#if canWrite}
			<Button
				variant="subtle"
				unifiedSize="xs"
				btnClasses={editing ? 'bg-surface-hover' : ''}
				startIcon={{ icon: Pen }}
				iconOnly
				title={editing ? 'Stop editing the description' : 'Edit the description'}
				aria-label={editing ? 'Stop editing the description' : 'Edit the description'}
				on:click={() => (editing = !editing)}
			/>
		{/if}
	</h4>
	{#if canWrite && editing}
		<div class="relative">
			<div class="text-2xs text-primary absolute -top-4 right-0">GH Markdown</div>
			<TextInput
				underlyingInputEl="textarea"
				bind:value={description}
				inputProps={{ placeholder, 'aria-label': label, disabled: !canWrite }}
			/>
		</div>
	{:else if description == undefined || description == ''}
		<div class="text-xs text-secondary font-normal">No description provided</div>
	{:else}
		<GfmMarkdown md={description} prose="sm" noPadding />
	{/if}
</div>
