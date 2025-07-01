<!--
	Useful to listen to changes on a list without recomputing the whole list
	when a single item changes
-->

<script lang="ts" generics="T">
	import { untrack } from 'svelte'

	let { key, onChange }: { key: T; onChange: () => void } = $props()

	$effect(() => {
		key
		untrack(() => onChange())
	})
</script>
