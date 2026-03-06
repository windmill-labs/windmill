<script lang="ts">
	import { urlize } from '$lib/utils'

	interface Props {
		text: string;
	}

	let { text }: Props = $props();
	let parsed = $derived(text ? urlize(text, 'html') : '')
</script>

{@html parsed}
