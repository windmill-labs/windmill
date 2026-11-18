<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- Pre-Oracle NetSuite "N" mark. #125580/#baccdb approximate netsuite.com's own 2014 logo art, which
	 is itself inconsistent: /portal/common/img/ns-logo.png is #14487e/#b9c9d5 and
	 /portal/common/img/logo-ns-mobile.png is #13527d/#b6c7d5 (both via web.archive.org/web/2014/).
	 Not Oracle's current NetSuite mark, which is a different logo in a different palette (#264759/#36677D/#94BFCE/#E2C06B). -->
<svg
	xmlns="http://www.w3.org/2000/svg"
	role="img"
	{width}
	{height}
	viewBox="1.42 1.4 21.161 21.161"
>
	<path
		d="m 3.2198637,7.809131 h 3.7965561 v 9.018821 h 1.8902684 v 3.52423 H 3.2198637 Z M 20.792951,16.107089 H 16.996396 V 7.088266 H 15.106127 V 3.5640368 h 5.686824 z"
		fill="#baccdb"
	/>
	<path
		d="M 2.3388066,2.6989989 H 14.337205 V 12.999359 L 9.7236683,7.0562275 H 2.3388066 Z M 21.65799,21.233239 H 9.6595914 V 10.93288 l 4.6135366,5.943131 h 7.384862"
		fill="#125580"
	/>
</svg>
