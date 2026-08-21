<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #533AFD tile / #FFFFFF bar per Stripe's own favicon.svg and Stripe_logo_kit.zip
	(stripe.com/newsroom/brand-assets). Stripe's Marks Usage Terms forbid altering the marks, so
	this ships verbatim in both themes rather than as a recoloured pair. -->
<svg
	{width}
	{height}
	viewBox="-23.273 -23.273 558.545 558.545"
	fill="none"
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	aria-label="Stripe"
>
	<rect width="512" height="512" rx="64" fill="#533AFD" />
	<path
		fill-rule="evenodd"
		clip-rule="evenodd"
		d="M120 392L392 334.317V120L120 178.357V392Z"
		fill="#FFFFFF"
	/>
</svg>
