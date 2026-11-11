<script lang="ts">
	interface Props {
		label: string
		value: number
		min: number
		max: number
		step: number
		format?: (value: number) => string
	}

	let { label, value = $bindable(), min, max, step, format = (v) => String(v) }: Props = $props()
</script>

<label class="flex flex-col gap-1">
	<span class="flex items-baseline justify-between gap-2 text-xs">
		<span class="font-semibold text-emphasis">{label}</span>
		<span class="tabular-nums text-secondary">{format(value)}</span>
	</span>
	<!-- `no-default-style` opts out of the app's global input styles, which draw a text box. -->
	<input
		type="range"
		class="no-default-style w-full [accent-color:rgb(var(--color-surface-accent-primary))]"
		{min}
		{max}
		{step}
		bind:value
	/>
</label>
