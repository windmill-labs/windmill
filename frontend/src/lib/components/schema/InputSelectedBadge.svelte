<script lang="ts">
	import { classes } from '$lib/components/common/alert/model'
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { X } from 'lucide-svelte'

	export let inputSelected: 'history' | 'captures' | 'saved' | undefined = undefined
</script>

<div class="min-h-[34px]">
	<div
		class="rounded-md flex flex-row gap-2 items-center py-1 px-2 w-fit {classes['info']
			.bgClass} {inputSelected ? '' : 'hidden'}"
	>
		<p class={twMerge(classes['info'].descriptionClass, 'text-xs px-2')}>
			Using {inputSelected === 'history'
				? 'historic'
				: inputSelected === 'captures'
				? 'captures'
				: 'saved'} input arguments
		</p>
		<Button
			color="light"
			size="xs2"
			startIcon={{ icon: X }}
			shortCut={{ key: 'esc', withoutModifier: true }}
			nonCaptureEvent
		/>
	</div>
</div>
