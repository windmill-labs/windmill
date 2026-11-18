<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	interface Props {
		size?: number
		height?: number
		width?: number
		color?: string | undefined
		class?: string
	}

	let {
		size = undefined,
		height: heightProp = 24,
		width: widthProp = 24,
		color = undefined,
		class: clazz = ''
	}: Props = $props()

	const { width, height } = $derived(
		size ? { width: size, height: size } : { width: widthProp, height: heightProp }
	)
</script>

<!-- #232220 / #FFFFFF per auth0.com docs logo light.svg / dark.svg. Okta's content terms forbid altering the mark, so ship only these published variants. -->
<svg
	role="img"
	{width}
	{height}
	viewBox="-3.198 -1.091 26.182 26.182"
	fill={color ?? 'currentColor'}
	xmlns="http://www.w3.org/2000/svg"
	class={twMerge('text-[#232220] dark:text-[#FFFFFF]', clazz)}
>
	<title>auth0-svg</title>
	<path
		d="M1.26834 10.2499C5.20746 9.60217 8.29463 6.51809 8.94341 2.58331L9.259 0.669875C9.31961 0.299735 9.01532 -0.0268029 8.64069 0.00174179C5.64144 0.23355 2.81491 1.2254 1.2539 1.86467C0.495844 2.17553 0 2.91142 0 3.73043V9.77877C0 10.1345 0.31842 10.4052 0.670128 10.3475L1.26834 10.2499Z"
	/>
	<path
		d="M10.8431 2.58304C11.4915 6.51782 14.579 9.60158 18.5181 10.2496L19.1163 10.3475C19.4681 10.4052 19.7865 10.1345 19.7865 9.77881V3.73047C19.7865 2.91115 19.2906 2.17557 18.5326 1.86472C16.9716 1.22419 14.1438 0.233592 11.1458 0.00178354C10.7712 -0.0270748 10.4656 0.298523 10.5275 0.669918L10.8431 2.58304Z"
	/>
	<path
		d="M18.5182 12.1463C14.5791 12.794 11.4919 15.8781 10.8431 19.8129L10.4497 23.5475C10.4136 23.8844 10.7885 24.1219 11.0708 23.9333C11.0736 23.9318 11.0752 23.9305 11.078 23.929C13.5485 22.2646 19.1839 17.9079 19.7416 12.4659C19.769 12.198 19.5313 11.9794 19.2659 12.0224L18.5192 12.1447L18.5182 12.1463Z"
	/>
	<path
		d="M8.94451 19.8092C8.29605 15.8744 5.20857 12.7906 1.26945 12.1426L0.466486 12.0102C0.230026 11.9713 0.018374 12.167 0.0412978 12.406C0.570115 17.8785 6.28347 22.2624 8.77557 23.9296C9.0337 24.1009 9.37504 23.8951 9.34207 23.5871L8.94451 19.8092Z"
	/>
</svg>
