<script>
	import { twMerge } from 'tailwind-merge'
</script>

<div class="pb-8 h-fit-content">
	<div class={twMerge('max-w-6xl mx-auto px-4 sm:px-6 md:px-8 h-fit-content', $$restProps.class)}>
		<slot />
	</div>
</div>
