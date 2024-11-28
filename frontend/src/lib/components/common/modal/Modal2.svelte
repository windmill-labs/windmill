<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import List from '$lib/components/common/layout/List.svelte'

	export let title: string

	export let css: any = {}
	export let target: string = ''
	export let mode: 'dnd' | 'fixed' = 'dnd'
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
</script>

<svelte:window on:keydown={handleKeyDown} />

<Portal name="always-mounted" {target}>
	<div
		class={twMerge(
			`${
				mode == 'dnd' ? 'absolute' : 'fixed'
			} top-0 bottom-0 left-0 right-0 transition-all duration-50 overflow-hidden`,
			isOpen ? 'z-[1100] bg-black bg-opacity-60' : 'hidden'
		)}
	>
		<div class="flex min-h-full items-center justify-center p-8">
			<div
				style={`width: ${sizeStyles[fixedSize].width}; height: ${sizeStyles[fixedSize].height}; ${
					css?.popup?.style || ''
				}`}
				class={twMerge(
					'max-h-screen-80 max-w-screen-80 rounded-lg relative bg-surface pt-2 px-4 pb-4',
					css?.popup?.class,
					'wm-modal-form-popup'
				)}
				use:clickOutside
				on:click_outside={() => {
					close()
				}}
			>
				<div class="h-full">
					<List gap="md">
						<div class="flex w-full">
							<List horizontal justify="between">
								<h3>{title}</h3>
								<div>
									<List horizontal justify="end">
										<slot name="header-right" />
										<div class="w-8">
											<button
												on:click={() => {
													isOpen = false
												}}
												class="hover:bg-surface-hover rounded-full w-8 h-8 flex items-center justify-center transition-all"
											>
												<X class="text-tertiary " />
											</button>
										</div>
									</List>
								</div>
							</List>
						</div>

						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<!-- svelte-ignore a11y-no-static-element-interactions -->
						<div class="w-full h-full flex grow min-h-0" on:click|stopPropagation={() => {}}>
							<div class="w-full">
								<slot />
							</div>
						</div>
					</List>
				</div>
			</div>
		</div>
	</div>
</Portal>
