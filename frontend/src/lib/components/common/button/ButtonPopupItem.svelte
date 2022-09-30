<script lang="ts">
	import { getContext } from 'svelte'
	import { Button, ButtonType } from '..'
	import { classNames } from '../../../utils'

	export let btnClasses: string = ''
	export let disabled: boolean = false
	export let href: string | undefined = undefined
	export let target: ButtonType.Target = '_self'
	export let iconOnly: boolean = false
	export let startIcon: ButtonType.Icon | undefined = undefined
	export let endIcon: ButtonType.Icon | undefined = undefined

	const props = getContext<ButtonType.ItemProps | undefined>(ButtonType.ItemContextKey)

	$: buttonProps = {
		...props,
		variant: <ButtonType.Variant>'border',
		btnClasses: classNames(btnClasses, '!border-0 !rounded-none !w-full'),
		disabled,
		href,
		target,
		iconOnly,
		startIcon,
		endIcon
	}
</script>

<li class="mt-1">
	<Button {...buttonProps}>
		<slot />
	</Button>
</li>
