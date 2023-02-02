<script lang="ts">
	import { page } from '$app/stores'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'
	import InlineCodeCopy from './InlineCodeCopy.svelte'

	$: opened = false
	$: workspace = $userWorkspaces.find((e) => e.id === $workspaceStore)
	$: workspaceName = workspace?.name
	$: workspaceId = workspace?.id
	$: url = `${$page.url.protocol}//${$page.url.hostname}`
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
		<ul class="pl-0 list-decimal list-inside">
			<li
				>Install the latest wmill CLI from deno.land: <InlineCodeCopy
					content={'deno install --unstable -A https://deno.land/x/wmill/main.ts'}
				/></li
			>
			<li
				>Setup the wmill cli for this workspace & remote: <InlineCodeCopy
					content={`wmill workspace add ${workspaceName} ${workspaceId} ${url}`}
				/></li
			>
			<li>Follow the prompts in your terminal</li>
			<li>Use the run command above!</li>
		</ul>
	</div>
{/if}
