<script lang="ts">
	interface Props {
		height?: string
		width?: string
	}

	let { height = '24px', width = '24px' }: Props = $props()
</script>

<!-- #FAD961 -> #F76B1C gradient per the logo in the meteosource.com site header (no brand page published). -->
<svg {width} {height} viewBox="-1.806 -4.806 42.614 42.614" xmlns="http://www.w3.org/2000/svg">
	<defs>
		<linearGradient x1="50%" y1="0%" x2="50%" y2="100%" id="meteosource-logo-meteosource-a">
			<stop stop-color="#FAD961" offset="0%"></stop>
			<stop stop-color="#F76B1C" offset="100%"></stop>
		</linearGradient>
	</defs><path
		fill="none"
		d="M6.948 29h23.62l.37-.035C34.924 28.585 38 25.204 38 21.14c0-4.342-3.496-7.86-7.806-7.86-.661 0-1.31.083-1.938.244l-1.097.282-.144-1.124C26.384 7.742 22.187 4 17.194 4 11.725 4 7.29 8.462 7.29 13.969c0 .529.041 1.052.122 1.567l.164 1.046-1.054.104C3.403 16.994 1 19.646 1 22.828c0 3.275 2.54 5.973 5.768 6.161l.18.011z"
		stroke="url(#meteosource-logo-meteosource-a)"
		stroke-width="2"
	>
	</path></svg
>
