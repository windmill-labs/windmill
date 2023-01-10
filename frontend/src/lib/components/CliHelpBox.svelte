<script lang="ts">
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'

	$: opened = false
</script>

<div class="text-sm mt-2 flex">
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<span
		class="underline mr-4"
		on:click={() => {
			opened = !opened
		}}
	>
		CLI quick setup
		<Icon class="ml-2" data={opened ? faChevronUp : faChevronDown} scale={0.7} />
	</span>
</div>

{#if opened}
	<div
		transition:slide|local
		class="bg-gray-100 border-l-4 border-gray-600 text-gray-700 p-4 m-4"
		role="alert"
		id="dynamic-input-help-box"
	>
		<ul class="ml-4">
			<li>FOO </li>
			<li>BAR</li>
		</ul>
	</div>
{/if}
