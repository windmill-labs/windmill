<script lang="ts">
	import { faChevronDown } from '@fortawesome/free-solid-svg-icons'
	import { setContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Button, ButtonType, Popup } from '..'

	export let size: ButtonType.Size = 'md'
	export let color: ButtonType.Color = 'blue'
	export let variant: ButtonType.Variant = 'contained'
	export let mainClasses: string = ''
	export let toggleClasses: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: ButtonType.Target = '_self'
	export let startIcon: ButtonType.Icon | undefined = undefined
	export let endIcon: ButtonType.Icon | undefined = undefined

	setContext<ButtonType.ItemProps>(ButtonType.ItemContextKey, { size, color })

	let ref: Popup['ref'],
		open: Popup['open'],
		close: Popup['close'],
		isOpen: Popup['isOpen'],
		isFocused: Popup['isFocused']

	const toggle = () => !isOpen && open()
	function conditionalClose() {
		setTimeout(() => {
			if (!isFocused) close()
		}, 0)
	}

	$: separator = color === 'red' || color === 'blue' ? 'border-gray-200' : 'border-gray-400'
	$: commonProps = {
		size,
		color,
		variant,
		disabled
	}
</script>

<div class="flex justy-start items-center">
	{#if $$slots.main}
		<Button
			{...commonProps}
			{href}
			{target}
			{startIcon}
			{endIcon}
			btnClasses="!rounded-r-none !border-r-0 {mainClasses}"
		>
			<slot name="main" />
		</Button>
	{/if}
	{#if ref}
		<span use:ref class={$$slots.main && variant === 'contained' ? 'border-l ' + separator : ''}>
			<Button
				on:click={toggle}
				{...commonProps}
				btnClasses="{$$slots.main ? '!rounded-l-none' : ''} {toggleClasses}"
				on:focus={open}
				on:blur={conditionalClose}
			>
				<slot name="toggle">
					<!-- Invisible, but needed to match the height of the 'main' button -->
					<span class="!opacity-0 !w-0">A</span>
					<Icon data={faChevronDown} scale={ButtonType.IconScale[size]} />
				</slot>
			</Button>
		</span>
	{/if}
</div>
<Popup
	bind:ref
	bind:open
	bind:close
	bind:isOpen
	bind:isFocused
	options={{
		placement: $$slots.main ? 'bottom-end' : 'bottom',
		strategy: 'absolute',
		modifiers: [{ name: 'offset', options: { offset: [0, 0] } }]
	}}
	outerClasses="shadow-lg rounded"
>
	<ul class="bg-white rounded-t border py-2 max-h-40 overflow-auto">
		<slot />
	</ul>
</Popup>
