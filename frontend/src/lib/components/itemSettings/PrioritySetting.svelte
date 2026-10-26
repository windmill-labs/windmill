<script lang="ts">
	import Label from '$lib/components/Label.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { enterpriseLicense } from '$lib/stores'
	import SettingToggle from './SettingToggle.svelte'

	interface Props {
		kind: 'flow' | 'script'
		priority: number | undefined
	}

	let { kind, priority = $bindable() }: Props = $props()

	// A priority of 0 reads as off, so emptying the field or typing 0 must not close it mid-edit.
	let editing = $state(false)
</script>

<!-- Not available on the cloud, where the toggle could only ever be disabled. -->
{#if !isCloudHosted()}
	<SettingToggle
		setting="priority"
		label="High priority"
		tooltip={kind === 'flow'
			? 'The jobs of this flow and of its steps go ahead of the other jobs in the queue.'
			: 'The jobs of this script go ahead of the other jobs in the queue.'}
		documentationLink={kind === 'flow'
			? 'https://www.windmill.dev/docs/flows/priority'
			: 'https://www.windmill.dev/docs/core_concepts/jobs#high-priority-jobs'}
		eeOnly
		disabled={!$enterpriseLicense}
		checked={editing || (priority != undefined && priority > 0)}
		onchange={(on) => {
			editing = on
			priority = on ? 100 : undefined
		}}
	>
		<Label label="Priority number">
			{#snippet header()}
				<Tooltip>From 0 to 100. Higher numbers run first.</Tooltip>
			{/snippet}
			<TextInput
				class="w-24"
				bind:value={priority}
				inputProps={{
					type: 'number',
					min: 0,
					max: 100,
					disabled: !$enterpriseLicense,
					onfocus: () => (editing = true),
					onchange: () => {
						if (priority != undefined && priority > 100) priority = 100
						else if (priority != undefined && priority < 0) priority = 0
					}
				}}
			/>
		</Label>
	</SettingToggle>
{/if}
