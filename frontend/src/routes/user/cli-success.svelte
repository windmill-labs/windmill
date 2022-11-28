<script lang="ts">
	import { goto } from '$app/navigation'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'

	import { sendUserToast } from '$lib/utils'
	import { faSpinner } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	sendUserToast('Sucess. You can return to your terminal now.')
	// goto('/')
</script>

<div class="my-grid">
	<div class="my-center">
		<div class="animate-spin"><WindmillIcon width="100%" height="100%" /></div>
	</div>
</div>

<style>
	.my-grid {
		display: grid;
		grid-template-areas: 'a a a' 'b c d' 'e e e';
		grid-auto-rows: 1fr;
		grid-auto-columns: 1fr;
		aspect-ratio: 1/1;
	}
	.my-center {
		grid-area: c;
	}
</style>
