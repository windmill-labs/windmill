<script lang="ts">

	interface Props {
		id: string;
	}

	let { id }: Props = $props();

	$effect(() => {
		id && console.log('updateJobId')
	});
</script>
