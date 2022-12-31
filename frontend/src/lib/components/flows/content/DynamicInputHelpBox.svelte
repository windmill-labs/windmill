<script lang="ts">
	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { slide } from 'svelte/transition'

	$: opened = false
</script>

<div class="text-xs flex flex-row-reverse">
	<span
		class="underline mr-4"
		on:click={() => {
			opened = !opened
		}}
	>
		Help
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
		the last line must always be the final expression.
		<p>
			If it is multiline, the statement before the final expression <b
				>MUST END WITH ; and a newline</b
			>
		</p>
		<ul class="ml-4">
			<li><b>{'results.<id>'}</b>: the result of step at id 'id'</li>
			<li><b>flow_input</b>: the object containing the flow input arguments</li>
			<li><b>params</b>: the object containing the current step static values</li>
			<li>
				<b>variable(path)</b>: the function returning the variable (including secrets) at given path
				as a string
			</li>
			<li>
				<b>resource(path)</b>: the function returning the resource at a given path as an object
			</li>
		</ul>
	</div>
{/if}
