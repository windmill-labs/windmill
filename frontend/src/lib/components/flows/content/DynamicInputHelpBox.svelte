<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { ChevronDown } from 'lucide-svelte'

	import { slide } from 'svelte/transition'

	$: opened = false
</script>

<div class="text-xs flex flex-row-reverse mt-1">
	<Button
		on:click={() => {
			opened = !opened
		}}
		color="light"
		size="xs2"
		endIcon={{ icon: ChevronDown, classes: `rotate-0 duration-300 ${opened ? '!rotate-180' : ''}` }}
	>
		Help
	</Button>
</div>

{#if opened}
	<div
		transition:slide|local
		class="bg-surface-secondary border-l-4 text-secondary p-4 m-4"
		role="alert"
		id="dynamic-input-help-box"
	>
		Single javascript expression. The following functions and objects are available:
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
		If using multiple statements, the last statement shall finish with a return. e.g: `return x`
	</div>
{/if}
