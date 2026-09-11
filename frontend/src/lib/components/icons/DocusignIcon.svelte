<!-- #000000 / #FFFFFF per brand.docusign.com/logo: only the Nexus overlap flips per background, Cobalt #4C00FF and Poppy #FF5252 must not be recoloured. -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg
	{width}
	{height}
	viewBox="-2.909 -2.909 69.818 69.818"
	xmlns="http://www.w3.org/2000/svg"
	class="text-[#000000] dark:text-[#FFFFFF]"
>
	<path
		d="M46.09,46.09V62.45c0,0.86-0.69,1.55-1.55,1.55H1.55c-0.86,0-1.55-0.69-1.55-1.55V19.49c0-0.86,0.69-1.55,1.55-1.55h16.32v26.6c0,0.86,0.69,1.55,1.55,1.55H46.09L46.09,46.09z"
		fill="#4C00FF"
	/>
	<path
		d="M64,23.03c0,13.74-8.03,23.03-17.91,23.06V28.78c0-0.5-0.2-0.99-0.56-1.32l-8.99-8.99c-0.36-0.36-0.83-0.56-1.32-0.56H17.91V1.59c0-0.86,0.69-1.55,1.55-1.55h24.22C55.18,0,64,9.25,64,23.03z"
		fill="#FF5252"
	/>
	<path
		d="M45.53,27.42c0.36,0.36,0.56,0.83,0.56,1.32v17.31H19.46c-0.86,0-1.55-0.69-1.55-1.55V17.91h17.31c0.5,0,0.99,0.2,1.32,0.56L45.53,27.42z"
		fill="currentColor"
	/>
</svg>
