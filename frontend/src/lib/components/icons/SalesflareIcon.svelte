<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #0053FF per salesflare.com's own `--color--major-blue` design token. -->
<svg {width} {height} viewBox="-1.457 -6.282 50.564 50.564" xmlns="http://www.w3.org/2000/svg">
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M22.32 5C18.454 5 15.32 8.13401 15.32 12C15.32 13.9741 13.9679 16.6667 11.9937 16.6667H8.81999C4.30966 16.6667 0.65332 20.323 0.65332 24.8333C0.65332 29.3437 4.30966 33 8.81999 33H26.5133C31.0236 33 34.68 29.3437 34.68 24.8333C34.68 22.4567 36.3503 19 38.727 19H40C43.866 19 47 15.866 47 12C47 8.13401 43.866 5 40 5H22.32Z"
		fill="#0053FF"
	/>
</svg>
