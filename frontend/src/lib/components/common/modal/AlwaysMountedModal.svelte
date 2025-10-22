<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'

	export let title: string
	export let style: string = ''
	export let css: any = {}

	const { mode } = getContext<AppViewerContext>('AppViewerContext')

	let isOpen = false

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
				use:clickOutside
				on:click_outside={() => {
					close()
				}}
			>
				<div class="px-4 py-2 border-b flex justify-between items-center">
					<div>{title}</div>
					<div class="w-8">
						<button
							on:click={() => {
								isOpen = false
							}}
							class="hover:bg-surface-hover bg-surface-secondary rounded-full w-8 h-8 flex items-center justify-center transition-all"
						>
							<X class="text-primary" />
						</button>
					</div>
				</div>

				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<div class="relative bg-surface rounded-md" on:click|stopPropagation={() => {}}>
					<div
						class={twMerge(
							'max-w-screen-lg max-h-screen-80 overflow-auto flex flex-col',
							$$props.class
						)}
						{style}
					>
						<slot />
					</div>
				</div>
			</div>
		</div>
	</div></Portal
>
