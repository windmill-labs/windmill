<script lang="ts">
	import { page } from '$app/stores'

	import { SvelteToast } from '@zerodevx/svelte-toast'

	// Default toast options
	const toastOptions = {
		duration: 4000, // duration of progress bar tween to the `next` value
		initial: 1, // initial progress bar value
		next: 0, // next progress value
		pausable: false, // pause progress bar tween on mouse hover
		dismissable: true, // allow dismiss with close button
		reversed: false, // insert new toast to bottom of stack
		intro: { x: 256 }, // toast intro fly animation settings
		theme: {} // css var overrides
	}
</script>

<svelte:head>
	<title>{$page.stuff.title ? `${$page.stuff.title} | ` : ''}Windmill</title>
</svelte:head>

<slot />
<SvelteToast options={toastOptions} />

<style>
	:root {
		--toastBackground: #eff6ff;
		--toastBarBackground: #eff6ff;
		--toastColor: #123456;
	}
</style>
