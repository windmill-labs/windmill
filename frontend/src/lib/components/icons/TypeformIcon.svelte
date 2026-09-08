<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #000000 / #FFFFFF per typeform.com/brand. Default brand colours are "Paper (white) and Ink (black)". -->
<svg
	class="text-[#000000] dark:text-[#FFFFFF]"
	x="0px"
	y="0px"
	{width}
	{height}
	viewBox="-1.5 -1.5 36.014 36.014"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
>
	<g transform="translate(0 5.67015)">
		<path
			fill-rule="evenodd"
			clip-rule="evenodd"
			d="M0 5.42456C0 1.8517 1.40765 0 3.78009 0C6.15215 0 7.56018 1.8517 7.56018 5.42456V16.2479C7.56018 19.8208 6.15252 21.6725 3.78009 21.6725C1.40765 21.6725 0 19.8208 0 16.2479V5.42456ZM25.4643 0H17.6512C10.6419 0 10.0894 3.027 10.0894 7.06301L10.0802 14.599C10.0802 18.8069 10.6082 21.6725 17.6784 21.6725H25.4643C32.4961 21.6725 33.0128 18.656 33.0128 14.62V7.07352C33.0128 3.027 32.4736 0 25.4643 0Z"
			fill="currentColor"
		/>
	</g>
</svg>
