<script lang="ts">
	interface Props {
		size?: number;
	}

	let { size = 24 }: Props = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
	<path
		d="M18.37 2.62999L14 6.99999L12.41 5.40999C12.0353 5.03749 11.5284 4.8284 11 4.8284C10.4716 4.8284 9.96473 5.03749 9.59 5.40999L8 6.99999L17 16L18.59 14.41C18.9625 14.0353 19.1716 13.5284 19.1716 13C19.1716 12.4716 18.9625 11.9647 18.59 11.59L17 9.99999L21.37 5.62999C21.7678 5.23216 21.9913 4.6926 21.9913 4.12999C21.9913 3.56738 21.7678 3.02781 21.37 2.62999C20.9722 2.23216 20.4326 2.00867 19.87 2.00867C19.3074 2.00867 18.7678 2.23216 18.37 2.62999Z"
		stroke="currentcolor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M9 8C7 11 5 11.5 2 12L10 22C12 21 16 17 16 15"
		stroke="currentcolor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M14.5 17.5L4.5 15"
		stroke="currentcolor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M22 22L2 2"
		stroke="currentcolor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
</svg>
