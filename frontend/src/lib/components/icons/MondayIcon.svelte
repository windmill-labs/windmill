<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #FB275D / #FFCC00 / #00CA72 per monday.com's official logo pack (brand-monday.com/logo). All three colours are required; the brand forbids monochrome or recoloured versions, so no dark-theme variant. -->
<svg {width} {height} viewBox="-2.909 -2.971 69.818 69.818" xmlns="http://www.w3.org/2000/svg">
	<g transform="matrix(3.208255 0 0 3.208255 -35.559129 -63.587202)"
		><path
			d="M13.513 35.76a2.433 2.433 0 0 1-2.059-3.723l4.377-6.99a2.432 2.432 0 1 1 4.123 2.582l-4.378 6.99a2.43 2.43 0 0 1-2.063 1.141z"
			fill="#FB275D"
		/><path
			d="M21.056 35.76a2.433 2.433 0 0 1-2.063-3.723l4.38-6.99a2.432 2.432 0 1 1 4.117 2.582l-4.372 6.99a2.43 2.43 0 0 1-2.063 1.141z"
			fill="#FFCC00"
		/><ellipse ry="2.375" rx="2.436" cy="33.384" cx="28.597" fill="#00CA72" /></g
	>
</svg>
