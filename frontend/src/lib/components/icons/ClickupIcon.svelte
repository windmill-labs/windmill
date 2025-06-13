<script lang="ts">
	interface Props {
		height?: string;
		width?: string;
	}

	let { height = '24px', width = '24px' }: Props = $props();
</script>

<svg
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="0 0 168 168"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M15 128.533L40.4652 108.979C53.9943 126.679 68.3685 134.838 84.3687 134.838C100.283 134.838 114.251 126.775 127.17 109.214L152.999 128.298C134.358 153.621 111.192 167 84.3687 167C57.6309 167 34.2405 153.706 15 128.533Z"
		fill="url(#paint0_linear_308_323)"
	/>
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M82.9336 43.5407L37.6076 82.693L16.6558 58.3354L83.0296 1L148.88 58.378L127.831 82.6503L82.9336 43.5407Z"
		fill="url(#paint1_linear_308_323)"
	/>
	<defs>
		<linearGradient
			id="paint0_linear_308_323"
			x1="15"
			y1="148.439"
			x2="152.999"
			y2="148.439"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#8930FD" />
			<stop offset="1" stop-color="#49CCF9" />
		</linearGradient>
		<linearGradient
			id="paint1_linear_308_323"
			x1="16.6558"
			y1="56.5595"
			x2="148.88"
			y2="56.5595"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#FF02F0" />
			<stop offset="1" stop-color="#FFC800" />
		</linearGradient>
	</defs>
</svg>
