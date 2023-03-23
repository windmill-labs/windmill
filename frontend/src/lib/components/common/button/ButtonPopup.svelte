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
	export let listClasses: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: ButtonType.Target = '_self'
	export let startIcon: ButtonType.Icon | undefined = undefined
	export let endIcon: ButtonType.Icon | undefined = undefined
	export let spacingSize: ButtonType.Size = size
	export let loading = false

	let ref: ButtonType.Element

	setContext<ButtonType.ItemProps>(ButtonType.ItemContextKey, { size, color })

	$: separator = color === 'red' || color === 'blue' ? 'border-gray-200' : 'border-gray-400'
	$: commonProps = {
		size,
		color,
		variant,
		disabled,
		spacingSize
	}
</script>

<div class="flex justy-start items-center">
	{#if $$slots.main}
		<Button
			{loading}
			{...commonProps}
			{href}
			{target}
			{startIcon}
			{endIcon}
			btnClasses="!rounded-r-none !border-r-0 {mainClasses}"
			on:click
		>
			<slot name="main" />
		</Button>
	{/if}
	<span class={$$slots.main && variant === 'contained' ? 'border-l ' + separator : ''}>
		<Button
			bind:element={ref}
			{...commonProps}
			btnClasses="{$$slots.main ? '!rounded-l-none' : ''} {toggleClasses}"
			on:click={() => {}}
		>
			<slot name="toggle">
				<!-- Invisible, but needed to match the height of the 'main' button -->
				<span class="!opacity-0 !w-0">A</span>
				<Icon data={faChevronDown} scale={ButtonType.IconScale[size]} />
			</slot>
		</Button>
	</span>
</div>
{#if ref}
	<Popup
		{ref}
		let:open
		let:close
		options={{
			placement: $$slots.main ? 'bottom-end' : 'bottom',
			strategy: 'absolute',
			modifiers: [{ name: 'offset', options: { offset: [0, 0] } }]
		}}
	>
		<ul class="bg-white rounded-t border pt-1 pb-2 max-h-40 overflow-auto {listClasses}">
			<slot {open} {close} />
		</ul>
	</Popup>
{/if}
