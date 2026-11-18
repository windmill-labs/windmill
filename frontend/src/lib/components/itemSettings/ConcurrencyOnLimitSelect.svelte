<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'

	interface Props {
		onLimit: 'queue' | 'fail' | undefined
		disabled?: boolean
	}

	let { onLimit = $bindable(), disabled = false }: Props = $props()
</script>

<Label label="When the limit is reached">
	<ToggleButtonGroup
		selected={onLimit ?? 'queue'}
		{disabled}
		class="w-auto"
		onSelected={(v) => (onLimit = v === 'fail' ? 'fail' : undefined)}
	>
		{#snippet children({ item })}
			<ToggleButton
				value="queue"
				label="Queue"
				tooltip="The run waits in the queue until a slot frees up."
				{item}
			/>
			<ToggleButton
				value="fail"
				label="Fail"
				tooltip="The run fails at once, with an error naming the concurrency key and the limit."
				{item}
			/>
		{/snippet}
	</ToggleButtonGroup>
</Label>
