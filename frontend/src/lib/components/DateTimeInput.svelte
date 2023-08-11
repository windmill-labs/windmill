<script lang="ts">
	export let value: string | undefined = undefined

	export let autofocus: boolean = false

	let dateFromValue: Date | undefined = value ? new Date(value) : undefined

	let date = isValidDate(dateFromValue) ? dateFromValue!.toISOString().split('T')[0] : undefined
	let time = isValidDate(dateFromValue) ? dateFromValue!.toISOString().split('T')[1] : '00:00'

	$: {
		if (date && time) {
			let newDate = new Date(`${date}T${time}`)
			value = newDate.toISOString()
		}
	}

	function isValidDate(d: Date | undefined): boolean {
		return d instanceof Date && !isNaN(d as any)
	}
</script>

<div class="flex flex-row gap-1 items-center w-full">
	<!-- svelte-ignore a11y-autofocus -->
	<input type="date" bind:value={date} {autofocus} class="!w-3/4" />
	<input type="time" bind:value={time} class="!w-1/4" />
</div>
