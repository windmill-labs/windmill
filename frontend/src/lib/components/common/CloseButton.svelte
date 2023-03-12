<script>
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'

	import { Icon } from 'svelte-awesome'

	export let noBg = false
	const dispatch = createEventDispatcher()
</script>

<button
	on:click={() => dispatch('close')}
	class="hover:bg-gray-200 {noBg
		? ''
		: 'bg-gray-100'} rounded-full w-8 h-8 flex items-center justify-center transition-all"
>
	<Icon data={faClose} class="text-gray-500" />
</button>
