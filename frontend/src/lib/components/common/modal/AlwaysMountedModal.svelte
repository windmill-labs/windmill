<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'

	interface Props {
		title: string
		style?: string
		css?: any
		class?: string
		children?: import('svelte').Snippet
	}

	let { title, style = '', css = {}, class: className = '', children }: Props = $props()

	const { mode } = getContext<AppViewerContext>('AppViewerContext')

	let isOpen = $state(false)

	export function close() {
		isOpen = false
	}

	export function open() {
		isOpen = true
	}
</script>

<Portal name="always-mounted" target="#app-editor-top-level-drawer">
	<div
		class={twMerge(
			`${
				$mode == 'dnd' ? 'absolute' : 'fixed'
			} top-0 bottom-0 left-0 right-0 transition-all duration-50 overflow-hidden`,
			isOpen ? 'z-[1100] bg-black bg-opacity-60' : 'hidden'
		)}
	>
		<div class="flex min-h-full items-center justify-center p-4">
			<div
				style={css?.popup?.style}
				class={twMerge(
					'bg-surface max-w-5xl m-24 overflow-y-auto rounded-lg relative',
					css?.popup?.class,
					'wm-modal-form-popup'
				)}
				use:clickOutside={{ onClickOutside: () => close() }}
			>
				<div class="px-4 py-2 border-b flex justify-between items-center">
					<div>{title}</div>
					<div class="w-8">
						<button
							onclick={() => {
								isOpen = false
							}}
							class="hover:bg-surface-hover bg-surface-secondary rounded-full w-8 h-8 flex items-center justify-center transition-all"
						>
							<X class="text-primary" />
						</button>
					</div>
				</div>

				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="relative bg-surface rounded-md" onclick={stopPropagation(() => {})}>
					<div
						class={twMerge(
							'max-w-screen-lg max-h-screen-80 overflow-auto flex flex-col',
							className
						)}
						{style}
					>
						{@render children?.()}
					</div>
				</div>
			</div>
		</div>
	</div></Portal
>
