<script lang="ts">
	import { urlize } from '$lib/utils'

	export let text: string
	$: parsed = text ? urlize(text, 'html') : ''
</script>

{@html parsed}
