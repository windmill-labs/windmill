<script lang="ts">
	import { run, createBubbler, stopPropagation, preventDefault } from 'svelte/legacy';

	const bubble = createBubbler();
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import { X } from 'lucide-svelte'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	interface Props {
		value?: any;
		placeholder?: string;
		type?: 'text' | 'textarea' | 'number';
		inputClass?: string;
		wrapperClass?: string;
		buttonClass?: string;
		children?: import('svelte').Snippet;
		[key: string]: any
	}

	let {
		value = $bindable(''),
		placeholder = '',
		type = 'text',
		inputClass = '',
		wrapperClass = '',
		buttonClass = '',
		children,
		...rest
	}: Props = $props();
	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)
	let isHovered = $state(false)

	let isNumeric = $derived(['number', 'range'].includes(type))
	run(() => {
		dispatchIfMounted('change', value)
	});

	function handleInput(e) {
		value = isNumeric ? +e.target.value : e.target.value
	}

	function clear() {
		value = ''
	}

	run(() => {
		if (value === undefined) value = ''
	});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="relative grow {wrapperClass}"
	onmouseenter={() => (isHovered = true)}
	onmouseleave={() => (isHovered = false)}
>
	{#if type === 'textarea'}
		<textarea
			{value}
			{placeholder}
			rows="1"
			class="resize-y {inputClass}"
			{...rest}
			oninput={handleInput}
			onkeydown={stopPropagation(bubble('keydown'))}
			onfocus={bubble('focus')}
			onblur={bubble('blur')}
		></textarea>
	{:else}
		<input
			{type}
			{value}
			{placeholder}
			class=" {(value ? '!pr-[26px] ' : '') + inputClass}"
			{...rest}
			oninput={handleInput}
			onkeydown={stopPropagation(bubble('keydown'))}
			onfocus={bubble('focus')}
			onblur={bubble('blur')}
		/>
	{/if}
	{#if value && isHovered}
		<button
			transition:fade|local={{ duration: 80 }}
			class="absolute z-10 top-[9.5px] right-2 rounded-full p-0.5 text-primary bg-surface-secondary
			 hover:bg-surface-hover focus:bg-surface-hover {buttonClass}"
			aria-label="Clear"
			onclick={stopPropagation(preventDefault(clear))}
		>
			<X size={11} class="" />
		</button>
	{/if}
	{@render children?.()}
</div>
