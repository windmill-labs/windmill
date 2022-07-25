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
		transition:slide
		class="bg-gray-100 border-l-4 border-gray-600 text-gray-700 p-4 m-4"
		role="alert"
		id="dynamic-input-help-box"
	>
		<p class="font-bold">Dynamic arg help</p>
		<p>
			When a field is "dynamic", its value is computed dynamically as the evaluation of its
			corresponding typescript snippet.
		</p>
		That snippet can be single line:
		<pre><code>last_result.myarg</code></pre>
		or multiline:
		<pre><code
				>let x = 5;
x + 2</code
			></pre>
		<p>
			If it is multiline, the last statement before the final expression<b
				>MUST END WITH ; and a newline</b
			>
		</p>
		The snippet can also be a string template:
		<pre><code
				>`Hello $&#123;params.name&#125;, all your base $&#123;previous_result.base_name&#125;
belong to us`</code
			></pre>
		However, the last line must always be the final expression.
		<p>
			The snippet can use any typescript primitives, and the following flow specific objects and
			functions:
		</p>
		<ul class="ml-4">
			<li>
				<b>previous_result</b>: the object containing the result of the previous step
			</li>
			<li><b>flow_input</b>: the object containing the flow input arguments</li>
			<li><b>params</b>: the object containing the current step static values</li>
			<li>
				<b>step(n)</b>: the function returning the result of step number n. One can also use a
				negative n. step(0) == result of the first step , step(-1) == previous_result
			</li>
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
