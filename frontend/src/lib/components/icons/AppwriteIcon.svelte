<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #FD366E ("Appwrite Pink") per https://appwrite.io/assets. Brand asks that the logo not be altered, so no per-theme variant. -->
<svg
	{width}
	{height}
	viewBox="-23.273 -23.223 558.545 558.545"
	fill="currentColor"
	xmlns="http://www.w3.org/2000/svg"
>
	<style>
		.appwrite-st0 {
			fill: #fd366e;
		}
	</style><path
		d="M512 368.9v112.9H225.2c-83.6 0-156.5-45.4-195.5-112.9-5.7-9.8-10.6-20.1-14.8-30.8C6.7 317.2 1.6 294.7 0 271.3v-30.5c.3-5.2.9-10.4 1.6-15.5C3 214.7 5.2 204.4 8 194.4 34.8 99.7 121.9 30.3 225.2 30.3s190.4 69.4 217.2 164.2H319.8c-20.1-30.9-55-51.3-94.6-51.3s-74.5 20.4-94.6 51.3c-6.1 9.4-10.9 19.7-14 30.8-2.8 9.8-4.2 20.1-4.2 30.8 0 32.4 13.6 61.5 35.4 82.1 20.2 19.1 47.5 30.8 77.4 30.8H512z"
		class="appwrite-st0"
	/><path
		d="M512 225.2v112.9H302.7c21.8-20.6 35.4-49.7 35.4-82.1 0-10.7-1.5-21-4.2-30.8z"
		class="appwrite-st0"
	/>
</svg>
