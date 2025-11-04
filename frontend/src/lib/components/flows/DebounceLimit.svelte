<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { SecondsInput } from '$lib/components/common'
	import { enterpriseLicense } from '$lib/stores'
	import Label from '../Label.svelte'

	let {
		debounce_delay_s = $bindable(),
		debounce_key = $bindable(),
		placeholder,
		size = 'xs',
		color = undefined,
		fontClass = 'font-normal'
	}: {
		debounce_delay_s: number | undefined
		debounce_key: string | undefined
		placeholder: string
		size: 'xs' | 'sm'
		color?: 'nord' | undefined
		fontClass?: string
	} = $props()
</script>

<div>
	<div class="flex flex-row items-center gap-2">
		<Toggle
			textClass={fontClass}
			{color}
			{size}
			disabled={!$enterpriseLicense}
			checked={Boolean(debounce_delay_s)}
			on:change={() => {
				if (debounce_delay_s) {
					debounce_delay_s = undefined
					debounce_key = undefined
				} else {
					debounce_delay_s = 1
				}
			}}
			options={{
				right: 'Debouncing',
				rightTooltip: 'Consolidate multiple executions into a single run within a time window',
				rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/job_debouncing'
			}}
			class="py-1"
			eeOnly={true}
		/>
	</div>

	{#if debounce_delay_s}
		<div class="flex flex-col gap-4 mt-2">
			<Label label="Delay in seconds">
				<SecondsInput disabled={!$enterpriseLicense} bind:seconds={debounce_delay_s} />
			</Label>
			<Label label="Custom debounce key (optional)">
				{#snippet header()}
					<Tooltip>
						Debounce keys are global, you can have them be workspace specific using the variable
						`$workspace`. You can also use an argument's value using `$args[name_of_arg]`</Tooltip
					>
				{/snippet}
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="text"
					autofocus
					disabled={!$enterpriseLicense}
					bind:value={debounce_key}
					{placeholder}
				/>
			</Label>
		</div>
	{/if}
</div>
