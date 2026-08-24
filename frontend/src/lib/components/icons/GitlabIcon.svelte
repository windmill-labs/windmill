<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		size?: number
		height?: number
		width?: number
		class?: string
	}

	let {
		size = undefined,
		height: heightProp = 24,
		width: widthProp = 24,
		class: clazz = ''
	}: Props = $props()

	const { width, height } = $derived(
		size ? { width: size, height: size } : { width: widthProp, height: heightProp }
	)
</script>

<!-- #E24329, with #FC6D26 / #FCA326, per https://design.gitlab.com/brand-design/color (Orange 03p/02p/01p, "colors from our core logo"). -->
<svg
	{width}
	{height}
	class={twMerge(clazz)}
	viewBox="85.24 85.26 209.509 209.509"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<defs
		><style>
			.gitlab-cls-2 {
				fill: #fc6d26;
			}
		</style></defs
	><g id="gitlab-LOGO"
		><path
			d="m282.83 170.73-.27-.69-26.14-68.22a6.8 6.8 0 0 0-2.69-3.24 7 7 0 0 0-8 .43 7 7 0 0 0-2.32 3.52l-17.65 54h-71.47l-17.65-54a6.86 6.86 0 0 0-2.32-3.53 7 7 0 0 0-8-.43 6.87 6.87 0 0 0-2.69 3.24L97.44 170l-.26.69a48.54 48.54 0 0 0 16.1 56.1l.09.07.24.17 39.82 29.82 19.7 14.91 12 9.06a8.07 8.07 0 0 0 9.76 0l12-9.06 19.7-14.91 40.06-30 .1-.08a48.56 48.56 0 0 0 16.08-56.04"
			style="fill:#e24329"
		/><path
			d="m282.83 170.73-.27-.69a88.3 88.3 0 0 0-35.15 15.8L190 229.25c19.55 14.79 36.57 27.64 36.57 27.64l40.06-30 .1-.08a48.56 48.56 0 0 0 16.1-56.08"
			class="gitlab-cls-2"
		/><path
			d="m153.43 256.89 19.7 14.91 12 9.06a8.07 8.07 0 0 0 9.76 0l12-9.06 19.7-14.91S209.55 244 190 229.25c-19.55 14.75-36.57 27.64-36.57 27.64"
			style="fill:#fca326"
		/><path
			d="M132.58 185.84A88.2 88.2 0 0 0 97.44 170l-.26.69a48.54 48.54 0 0 0 16.1 56.1l.09.07.24.17 39.82 29.82L190 229.21Z"
			class="gitlab-cls-2"
		/></g
	>
</svg>
