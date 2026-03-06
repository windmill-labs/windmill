<script lang="ts">
	import { ScanIcon } from 'lucide-svelte'

	interface Props {
		size?: number;
		variant?: 'redDot' | undefined;
	}

	let { size = 14, variant = undefined }: Props = $props();
</script>

<div class="relative">
	<ScanIcon {size} />
	<div class="absolute inset-0 flex items-center justify-center">
		<svg
			class={variant === 'redDot' ? 'text-red-500' : ''}
			width={size}
			height={size}
			viewBox="0 0 24 24"
			fill="currentColor"
			xmlns="http://www.w3.org/2000/svg"
		>
			<circle cx="12" cy="12" r="3" />
		</svg>
	</div>
</div>
