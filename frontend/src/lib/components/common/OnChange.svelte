<!--
	DEPRECATED: Use nested $effect or watch() from 'runed' instead.
-->

<script lang="ts" generics="T">
	import { untrack } from 'svelte'

	let {
		key,
		onChange,
		runFirstEffect = false
	}: {
		key: T
		onChange: () => void
		runFirstEffect?: boolean
	} = $props()

	let isFirstRun = true
	$effect(() => {
		key
		if (isFirstRun) {
			isFirstRun = false
			if (!runFirstEffect) return
		}
		untrack(() => onChange())
	})
</script>
