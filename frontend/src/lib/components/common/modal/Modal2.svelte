<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import List from '$lib/components/common/layout/List.svelte'
	import { fade } from 'svelte/transition'

	export let title: string

	export let css: any = {}
	export let target: string = ''
	export let isOpen = false
	export let fixedSize: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'xxl' = 'md'

	// Add size mapping with custom pixel values
	const sizeStyles = {
		xs: { width: '400px', height: '250px' },
		sm: { width: '600px', height: '400px' },
		md: { width: '800px', height: '500px' },
		lg: { width: '1400px', height: '720px' },
		xl: { width: '1600px', height: '800px' },
		xxl: { width: '1600px', height: '1000px' }
	}

	export function close() {
		isOpen = false
	}

	export function open() {
		isOpen = true
	}

	function handleKeyDown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault()
			event.stopPropagation()
			close()
		}
	}

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 200 })
	}
</script>

<svelte:window on:keydown={handleKeyDown} />

{#if isOpen}
	<Portal name="always-mounted" {target}>
		<div
			class={'fixed top-0 bottom-0 left-0 right-0 transition-all z-[1102] overflow-auto bg-black bg-opacity-60 w-full h-full'}
			transition:fadeFast|local
		>
			<div class="flex min-h-full items-center justify-center p-8">
				<div
					style={`width: ${sizeStyles[fixedSize].width}; height: ${sizeStyles[fixedSize].height}; ${
						css?.popup?.style || ''
					}`}
					class={twMerge(
						'max-h-screen-80 max-w-screen-80 rounded-lg relative bg-surface p-4',
						css?.popup?.class,
						'wm-modal-form-popup'
					)}
					use:clickOutside
					on:click_outside={() => {
						close()
					}}
				>
					<List gap="md">
						<div class="flex w-full">
							<List horizontal justify="between">
								<h3>{title}</h3>
								<div class="grow w-min-0">
									<List horizontal justify="between">
										<div class="min-w-0 grow">
											<slot name="header-left" />
										</div>
										<div class="min-w-0 grow-0 justify-end">
											<List horizontal justify="end">
												<slot name="header-right" />
												<div class="w-8">
													<button
														on:click={() => {
															isOpen = false
														}}
														class="hover:bg-surface-hover rounded-full w-8 h-8 flex items-center justify-center transition-all"
													>
														<X class="text-primary " />
													</button>
												</div>
											</List>
										</div>
									</List>
								</div>
							</List>
						</div>

						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<!-- svelte-ignore a11y-no-static-element-interactions -->
						<div class="w-full flex grow min-h-0" on:click|stopPropagation={() => {}}>
							<slot />
						</div>
					</List>
				</div>
			</div>
		</div>
	</Portal>
{/if}
