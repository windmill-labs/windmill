<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ChevronDown } from 'lucide-svelte'

	let opened = $state(false)
</script>

<!-- Pulled up against the input above: it reads as a footnote on that field rather than
     as the next thing in the form. -->
<div class="text-2xs flex flex-row-reverse -mt-2">
	<Button
		on:click={() => {
			opened = !opened
		}}
		color="light"
		variant="divider"
		size="xs2"
		endIcon={{ icon: ChevronDown, classes: `rotate-0 duration-300 ${opened ? '!rotate-180' : ''}` }}
		btnClasses="text-hint !text-2xs font-normal py-0"
	>
		Help
	</Button>
</div>

{#if opened}
	<Alert title="Single JavaScript expression" class="mt-2">
		The following functions and objects are available:
		<ul class="ml-4 list-disc">
			<li
				><b>{'results.<id>'}</b>: the result of step at id 'id' (use optional chaining if id may not
				exist because branch was not chosen, for example: <code>{'results.<id>?.[0]'}</code>)</li
			>
			<li><b>flow_input</b>: the object containing the flow input arguments</li>
			<li><b>previous_result</b>: the result of previous step</li>
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
	</Alert>
{/if}
