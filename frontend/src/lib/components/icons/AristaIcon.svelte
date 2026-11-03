<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- The "A" of Arista's wordmark, in its navy; white on dark grounds. -->
<svg
	class="text-[#16325B] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-8 -33 258 258"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Arista"
>
	<path
		d="M104.325,11.972C97.133,23.272,1.564,174.334,1.564,174.334h33.91l31.858-51.381h67.822l17.47-27.746H84.8l36.993-59.602l86.321,137.697h33.911c0,0-94.542-149.001-102.765-162.359C130.013-2.413,113.574-1.387,104.325,11.972z"
	/>
</svg>
