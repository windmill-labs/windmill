<script lang="ts">
	export let value: string | undefined = undefined

	export let autofocus: boolean = false

	let dateFromValue = value ? new Date(value) : undefined
	let date = dateFromValue ? dateFromValue.toISOString().split('T')[0] : undefined
	let time = dateFromValue ? dateFromValue.toISOString().split('T')[1] : undefined

	$: {
		if (date && time) {
			let newDate = new Date(`${date}T${time}`)
			value = newDate.toISOString()
		}
	}
</script>

<div class="flex flex-row gap-1 items-center w-full">
	<!-- svelte-ignore a11y-autofocus -->
	<input type="date" bind:value={date} {autofocus} class="!w-3/4" />
	<input type="time" bind:value={time} class="!w-1/4" />
</div>
