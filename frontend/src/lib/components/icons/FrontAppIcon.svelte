<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #A857F1 per the logo mark front.com ships inline on its own pages; the mark keeps this purple on both light and dark backgrounds. -->
<svg
	{width}
	{height}
	viewBox="-2.955 -3.018 70.909 70.909"
	fill="#A857F1"
	xmlns="http://www.w3.org/2000/svg"
>
	<path d="M0 63.6152H21.3918V21.3008H64.0428V0H0V63.6152Z" />
	<path
		d="M45.5945 64.8878C56.2523 64.8878 64.8922 56.2479 64.8922 45.5901C64.8922 34.9323 56.2523 26.2925 45.5945 26.2925C34.9367 26.2925 26.2969 34.9323 26.2969 45.5901C26.2969 56.2479 34.9367 64.8878 45.5945 64.8878Z"
	/>
</svg>
