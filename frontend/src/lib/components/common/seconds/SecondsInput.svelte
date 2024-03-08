<script lang="ts">
	type TimeUnit = number | undefined

	const ONE_DAY_IN_SECONDS = 86400 as const
	const ONE_HOUR_IN_SECONDS = 3600 as const
	const ONE_MINUTE_IN_SECONDS = 60 as const

	export let seconds = 0
	export let hideDisplay = false
	export let disabled = false
	export let max: number | undefined = undefined

	let day: TimeUnit = undefined
	let hour: TimeUnit = undefined
	let min: TimeUnit = undefined
	let sec: TimeUnit = undefined

	$: convertSecondsToTime(seconds)

	function convertSecondsToTime(seconds) {
		day = Math.floor(seconds / ONE_DAY_IN_SECONDS)
		seconds -= day * ONE_DAY_IN_SECONDS
		day = day || undefined

		hour = Math.floor(seconds / ONE_HOUR_IN_SECONDS)
		seconds -= hour * ONE_HOUR_IN_SECONDS
		hour = hour || undefined

		min = Math.floor(seconds / ONE_MINUTE_IN_SECONDS)
		seconds -= min * ONE_MINUTE_IN_SECONDS
		min = min || undefined

		sec = seconds || undefined
	}

	function convertUnitsToSeconds() {
		seconds =
			(day || 0) * ONE_DAY_IN_SECONDS +
			(hour || 0) * ONE_HOUR_IN_SECONDS +
			(min || 0) * ONE_MINUTE_IN_SECONDS +
			(sec || 0)
		if (seconds < 0) seconds = 0
		if (max && seconds > max) {
			seconds = max
		}
	}
</script>

<div class="flex flex-wrap gap-x-4">
	{#if !hideDisplay}
		<input
			value={seconds == null || seconds == undefined
				? 'Not set'
				: disabled
				? ''
				: seconds + ' second' + (seconds === 1 ? '' : 's')}
			{disabled}
			readonly
			type="text"
			class="max-w-[248px] bg-gray-50 mb-2 mt-6"
		/>
	{/if}
	<div class="flex flex-wrap items-center gap-2 text-xs font-medium">
		<div class="flex items-center gap-2">
			<label>
				Sec
				<input
					type="number"
					class="!w-14"
					{disabled}
					bind:value={sec}
					on:change={convertUnitsToSeconds}
					on:focus
				/>
			</label>
			<label>
				Min
				<input
					type="number"
					class="!w-14"
					{disabled}
					bind:value={min}
					on:change={convertUnitsToSeconds}
					on:focus
				/>
			</label>
		</div>
		<div class="flex items-center gap-2">
			<label>
				Hour
				<input
					type="number"
					class="!w-14"
					{disabled}
					bind:value={hour}
					on:change={convertUnitsToSeconds}
					on:focus
				/>
			</label>
			<label>
				Day
				<input
					type="number"
					class="!w-14"
					{disabled}
					bind:value={day}
					on:change={convertUnitsToSeconds}
					on:focus
				/>
			</label>
		</div>
	</div>
</div>
