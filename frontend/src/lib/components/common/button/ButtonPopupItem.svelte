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
	export let wrapperClasses = ''

	const props = getContext<ButtonType.ItemProps | undefined>(ButtonType.ItemContextKey)
	const iconWidthClass: Record<ButtonType.Size, string> = {
		xs: '!w-[12px]',
		sm: '!w-[14px]',
		md: '!w-[16px]',
		lg: '!w-[18px]',
		xl: '!w-[20px]'
	}

	const getWidthClass = () => (props?.size ? iconWidthClass[props.size] : undefined)

	$: buttonProps = {
		...props,
		variant: <ButtonType.Variant>'border',
		btnClasses: classNames(btnClasses, '!justify-start !border-0 !rounded-none !w-full'),
		disabled,
		href,
		target,
		iconOnly,
		startIcon: startIcon
			? {
					icon: startIcon.icon,
					classes: classNames(startIcon?.classes, getWidthClass())
			  }
			: undefined,
		endIcon: endIcon
			? {
					icon: endIcon.icon,
					classes: classNames(endIcon?.classes, getWidthClass())
			  }
			: undefined
	}
</script>

<li class="mt-1 {wrapperClasses}">
	<Button {...buttonProps} on:click>
		<slot />
	</Button>
</li>
