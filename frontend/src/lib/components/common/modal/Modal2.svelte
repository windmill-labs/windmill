<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import List from '$lib/components/common/layout/List.svelte'
	import { fade } from 'svelte/transition'

	interface Props {
		title: string
		css?: any
		target?: string
		isOpen?: boolean
		fixedWidth?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'xxl'
		fixedHeight?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | 'xxl'
		contentClasses?: string
		header_left?: import('svelte').Snippet
		header_right?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		title,
		css = {},
		target = '',
		isOpen = $bindable(false),
		fixedWidth = 'md',
		fixedHeight = 'md',
		contentClasses = '',
		header_left,
		header_right,
		children
	}: Props = $props()

	const widthMap = {
		xs: '400px',
		sm: '600px',
		md: '800px',
		lg: '1400px',
		xl: '1600px',
		xxl: '1600px'
	}
	const heightMap = {
		xs: '250px',
		sm: '400px',
		md: '500px',
		lg: '720px',
		xl: '800px',
		xxl: '1000px'
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

<svelte:window onkeydown={handleKeyDown} />

{#if isOpen}
	<Portal name="always-mounted" {target}>
		<div
			class={'fixed top-0 bottom-0 left-0 right-0 transition-all z-[1110] overflow-auto bg-black bg-opacity-60 w-full h-full'}
			transition:fadeFast
		>
			<div class="flex min-h-full items-center justify-center p-8">
				<div
					style={`width: ${widthMap[fixedWidth]}; height: ${heightMap[fixedHeight]}; ${
						css?.popup?.style || ''
					}`}
					class={twMerge(
						'max-h-screen-80 max-w-screen-80 rounded-lg relative bg-surface p-4',
						css?.popup?.class,
						'wm-modal-form-popup'
					)}
					use:clickOutside={{ onClickOutside: () => close() }}
				>
					<List gap="md">
						<div class="flex w-full">
							<List horizontal justify="between">
								<h3>{title}</h3>
								<div class="grow w-min-0">
									<List horizontal justify="between">
										<div class="min-w-0 grow">
											{@render header_left?.()}
										</div>
										<div class="min-w-0 grow-0 justify-end">
											<List horizontal justify="end">
												{@render header_right?.()}
												<div class="w-8">
													<button
														id="modal-close-button"
														onclick={() => {
															close()
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

						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="w-full flex grow min-h-0 {contentClasses}"
							onclick={(e) => e.stopPropagation()}
						>
							{@render children?.()}
						</div>
					</List>
				</div>
			</div>
		</div>
	</Portal>
{/if}
