<!-- #1A82E2 (SendGrid Blue) with logo accents #00B3E3 / #009DD9 / #99E1F4 per styleguide.sendgrid.com/colors.html. Twilio's trademark guidelines forbid recolouring the mark, so it stays multicolour in both themes. -->
<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<svg {width} {height} viewBox="-1.091 -1.091 26.182 26.182" xmlns="http://www.w3.org/2000/svg">
	<rect x="8" y="0" width="8" height="8" fill="#00B3E3" />
	<rect x="16" y="0" width="8" height="8" fill="#1A82E2" />
	<rect x="0" y="8" width="8" height="8" fill="#99E1F4" />
	<rect x="8" y="8" width="8" height="8" fill="#009DD9" />
	<rect x="16" y="8" width="8" height="8" fill="#00B3E3" />
	<rect x="0" y="16" width="8" height="8" fill="#1A82E2" />
	<rect x="8" y="16" width="8" height="8" fill="#99E1F4" />
</svg>
