<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import WorkerTagPicker from '$lib/components/WorkerTagPicker.svelte'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		tag: string | undefined
		/** Flows only: let steps that declare their own tag keep it. */
		preserveStepTags?: boolean | undefined
		placeholder?: string
		workspaceId?: string
	}

	let {
		kind,
		tag = $bindable(),
		preserveStepTags = $bindable(),
		placeholder,
		workspaceId
	}: Props = $props()

	// The picker opens before a tag is picked, so the toggle cannot read only `tag`.
	let picking = $state(false)
	let checked = $derived(picking || !!tag)
</script>

<SettingToggle
	setting="worker-group"
	label="Worker group tag"
	tooltip={kind === 'flow'
		? "Every step of the flow runs on the workers listening to this tag (queue), whatever the step's own tag. Without one, the flow's control steps run with the default 'flow' tag and each step with its own."
		: 'The script runs on the workers listening to this tag (queue), for instance a highmem or gpu worker group.'}
	documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
	{checked}
	onchange={(on) => {
		picking = on
		if (!on) {
			tag = undefined
			preserveStepTags = undefined
		}
	}}
>
	<WorkerTagPicker bind:tag {placeholder} {workspaceId} />
	{#if kind === 'flow' && tag}
		<Toggle
			size="2xs"
			bind:checked={() => preserveStepTags, (v) => (preserveStepTags = v || undefined)}
			options={{
				right: 'Preserve step worker tags',
				rightTooltip:
					'Steps that declare their own worker tag run on it instead of the flow tag. Steps without one still inherit the flow tag.',
				rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/worker_groups'
			}}
		/>
	{/if}
</SettingToggle>
