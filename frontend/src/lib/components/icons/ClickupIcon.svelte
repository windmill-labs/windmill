<!-- #FF02F0→#F76808 / #6647F0→#0091FF per clickup.com/brand (v4 Logomark-gradient.svg); the gradient mark is the same on light and dark, and the guidelines say "don't change the color". -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="-2.364 -2.114 68.727 68.727"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M6.43828 50.2095C5.81176 49.4456 5.94882 48.3212 6.71918 47.7027L14.0169 41.8433C14.6535 41.3322 15.5841 41.4476 16.1032 42.0777C21.06 48.0956 26.32 50.8815 32.142 50.8815C37.9238 50.8815 43.0403 48.135 47.7801 42.1839C48.2888 41.5452 49.2172 41.4131 49.8629 41.9129L57.2604 47.6377C58.0415 48.2422 58.1989 49.3637 57.5855 50.1379C50.5775 58.9839 41.9971 63.6446 32.142 63.6446C22.3156 63.6446 13.6586 59.013 6.43828 50.2095Z"
		fill="url(#clickup_paint0_linear)"
	/>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M32.4953 17.1804C32.2202 16.9368 31.8067 16.9367 31.5314 17.1801L15.663 31.2116C15.0564 31.748 14.1285 31.6857 13.599 31.0731L7.40986 23.9122C6.88774 23.3082 6.95019 22.3961 7.54979 21.8688L31.0527 1.20062C31.6021 0.717503 32.4248 0.717611 32.9741 1.20088L56.4822 21.8847C57.0822 22.4125 57.1439 23.3256 56.6205 23.9294L50.4164 31.0866C49.8862 31.6982 48.9589 31.7594 48.353 31.2229L32.4953 17.1804Z"
		fill="url(#clickup_paint1_linear)"
	/>
	<defs>
		<linearGradient
			id="clickup_paint0_linear"
			x1="5.3335"
			y1="32.1624"
			x2="58.6653"
			y2="32.1624"
			gradientUnits="userSpaceOnUse"
		>
			<stop offset="0.225962" stop-color="#6647F0" />
			<stop offset="0.793269" stop-color="#0091FF" />
		</linearGradient>
		<linearGradient
			id="clickup_paint1_linear"
			x1="5.3335"
			y1="31.6447"
			x2="58.6653"
			y2="31.6447"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#FF02F0" />
			<stop offset="0.778846" stop-color="#F76808" />
		</linearGradient>
	</defs>
</svg>
