<script lang="ts">
	import { run } from 'svelte/legacy';

	interface Props {
		id: string;
	}

	let { id }: Props = $props();

	run(() => {
		id && console.log('updateJobId')
	});
</script>
