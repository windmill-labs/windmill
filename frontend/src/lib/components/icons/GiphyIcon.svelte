<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #FFF35C / #00FF99 / #00CCFF / #9933FF / #FF6666 per GIPHY's own app icon (giphy.com/static/img/icons/apple-touch-icon-180px.png). Same full-colour mark on light and dark. -->
<svg {width} {height} viewBox="-1.091 -1.091 26.182 26.182" xmlns="http://www.w3.org/2000/svg">
	<path fill="#FFF35C" d="M2.666 0H12.666L10 2.668H5.334Z" />
	<path fill="#00FF99" d="M2.666 0L5.334 2.668V21.334L2.666 24Z" />
	<path fill="#00CCFF" d="M2.666 24L5.334 21.334H18.666L21.334 24Z" />
	<path fill="#9933FF" d="M21.334 24L18.666 21.334V11.334L21.334 8.666Z" />
	<path fill="#FF6666" d="M13.334 0V8H21.334V5.334H18.666V2.668H16V0Z" />
</svg>
