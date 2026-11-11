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

	// Any negative value sorts after jobs without priority, which are stored as 0.
	const LOW_PRIORITY = -100
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
		checked={priority !== undefined && priority > 0}
		onchange={(on) => (priority = on ? 100 : undefined)}
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
					onchange: () => {
						if (priority != undefined && priority > 100) priority = 100
						else if (priority != undefined && priority < 0) priority = 0
					}
				}}
			/>
		</Label>
	</SettingToggle>
	<SettingToggle
		setting="low_priority"
		label="Low priority"
		tooltip={kind === 'flow'
			? 'The jobs of this flow, of its steps and every job they start run only when no other job is waiting. Use it for large background flows so they do not hold up the rest of the workspace. A script that waits for a job it started keeps its worker until that job runs.'
			: 'The jobs of this script and every job they start run only when no other job is waiting. A script that waits for a job it started keeps its worker until that job runs.'}
		eeOnly
		disabled={!$enterpriseLicense}
		checked={priority !== undefined && priority < 0}
		onchange={(on) => (priority = on ? LOW_PRIORITY : undefined)}
	/>
{/if}
