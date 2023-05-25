<script lang="ts">
	import { page } from '$app/stores'
	import { workspaceStore } from '$lib/stores'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'
	import InlineCodeCopy from './InlineCodeCopy.svelte'

	$: opened = false
	$: url = `${$page.url.protocol}//${$page.url.hostname}/`
</script>

<div class="text-sm mt-2 flex">
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<button
		class="underline mr-4 text-gray-600"
		on:click={() => {
			opened = !opened
		}}
	>
		CLI quick setup
		<Icon class="ml-2" data={opened ? faChevronUp : faChevronDown} scale={0.7} />
	</button>
</div>

{#if opened}
	<div
		transition:slide|local={{ duration: 100 }}
		class="border border-gray-200 rounded-md p-4 mt-2 text-sm text-gray-600"
		role="alert"
		id="dynamic-input-help-box"
	>
		<ul class="pl-0 pt-2 list-decimal list-inside">
			<li
				>Install the latest wmill CLI from deno.land: <InlineCodeCopy
					content={'deno install --unstable -A https://deno.land/x/wmill/main.ts'}
				/></li
			>
			<li
				>Setup the wmill cli for this workspace & remote: <InlineCodeCopy
					content={`wmill workspace add ${workspaceStore} ${workspaceStore} ${url}`}
				/></li
			>
			<li>Follow the prompts in your terminal</li>
			<li>Use the run command above!</li>
		</ul>
	</div>
{/if}
