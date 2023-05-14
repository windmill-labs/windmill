<script lang="ts">
	import {
		Dialog,
		DialogOverlay,
		DialogTitle,
		Transition,
		TransitionChild
	} from '@rgossiaux/svelte-headlessui'
	import Portal from 'svelte-portal'
	import { twMerge } from 'tailwind-merge'
	import Button from '../button/Button.svelte'
	import Badge from '../badge/Badge.svelte'

	export let title: string
	export let style: string = ''

	let isOpen = false

	export function close() {
		isOpen = false
	}

	export function open() {
		isOpen = true
	}
</script>

<Portal target="#app-editor-top-level-drawer">
	<Transition show={isOpen}>
		<Dialog as="div" class="absolute inset-0 overflow-y-auto z-50" on:close={close}>
			<div class="min-h-screen px-4 text-center">
				<TransitionChild
					enter="ease-out duration-100"
					enterFrom="opacity-0"
					enterTo="opacity-100"
					leave="ease-in duration-100"
					leaveFrom="opacity-100"
					leaveTo="opacity-0"
				>
					<DialogOverlay class="fixed inset-0 bg-gray-800/50 " />
				</TransitionChild>

				<TransitionChild
					enter="ease-out duration-[50ms]"
					enterFrom="opacity-0 scale-95"
					enterTo="opacity-100 scale-100"
					leave="ease-in dduration-[50ms]"
					leaveFrom="opacity-100 scale-100"
					leaveTo="opacity-0 scale-95"
				>
					<!-- This element is to trick the browser into centering the modal contents. -->
					<span class="inline-block h-screen align-middle" aria-hidden="true"> &#8203; </span>
					<div
						class="inline-block w-full max-w-md p-6 text-left align-middle transition-all transform bg-white shadow-xl rounded-md"
					>
						<DialogTitle class="text-lg font-medium leading-6 text-gray-900">
							{title}
						</DialogTitle>
						<!-- svelte-ignore a11y-click-events-have-key-events -->
						<div class="relative bg-white rounded-md" on:click|stopPropagation={() => {}}>
							<div
								class={twMerge(
									'max-w-screen-lg max-h-screen-80 overflow-auto flex flex-col',
									$$props.class
								)}
								{style}
							>
								<div class="flex-1">
									<slot />
								</div>
								<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
									<Button on:click={close} color="light" size="sm">
										<span class="gap-2">Cancel <Badge color="dark-gray">Escape</Badge></span>
									</Button>
								</div>
							</div>
						</div>
					</div>
				</TransitionChild>
			</div>
		</Dialog>
	</Transition>
</Portal>
