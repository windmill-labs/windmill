<script lang="ts">
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
</script>

<button
	on:click
	type="button"
	class="text-gray-900 bg-white border m-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
>
	<Icon data={faPlus} scale={0.8} />
</button>
