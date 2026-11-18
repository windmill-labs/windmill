<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per personio.design/brand/how-we-look/logo. Black on light, white on dark or coloured backgrounds. -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	{width}
	{height}
	viewBox="-0.017 -0.017 24.035 24.035"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<path
		d="M14.0215 1C14.71 1.00006 15.3453 1.37237 15.6816 1.97266L17.6631 5.50684C17.9007 5.93018 18.3483 6.19225 18.834 6.19238H21.6582C22.3985 6.19253 22.9998 6.79213 23 7.53223V16.6416C23 17.3812 22.3993 17.9813 21.6582 17.9814H18.8027C18.3176 17.9815 17.8695 18.2442 17.6318 18.667L15.7471 22.0273C15.4107 22.6277 14.7742 23 14.085 23H8.35938V17.333H15.9834C16.724 17.333 17.3252 16.7336 17.3252 15.9932V8.00781C17.3252 7.26808 16.7247 6.66699 15.9834 6.66699H8.01758C7.27564 6.66699 6.67383 7.26872 6.67383 8.00977V15.4932H1V7.14648C1.00011 3.7518 3.75459 1.00017 7.15332 1H14.0215Z"
	/>
</svg>
