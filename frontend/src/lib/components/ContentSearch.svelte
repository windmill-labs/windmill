<script lang="ts">
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import Portal from 'svelte-portal'
	import { twMerge } from 'tailwind-merge'
	import ContentSearchInner from './ContentSearchInner.svelte'
	import { tick } from 'svelte'


	let contentSearchInner: ContentSearchInner | undefined = undefined
	let inputElement: HTMLInputElement

	export async function open(nsearch?: string) {
		isOpen = true
		await tick()
		await contentSearchInner?.open(nsearch)
		inputElement.focus()
	}

	let isOpen = false
	let search: string = ''

</script>

{#if isOpen}
	<Portal>
		<div
			class={twMerge(
				`fixed top-0 bottom-0 left-0 right-0 transition-all duration-50`,
				' bg-black bg-opacity-60',
				'z-[1100]'
			)}
		>
			<div
				class={'max-w-4xl lg:mx-auto mx-10 mt-8 bg-surface rounded-lg relative'}
				use:clickOutside={false}
				on:click_outside={() => {
					isOpen = false
				}}
			>
				<div class="px-4 py-2 border-b flex justify-between items-center">
					<div>Search by content</div>
					<div class="w-8">
						<button
							on:click|stopPropagation={() => {
								isOpen = false
							}}
							class="hover:bg-surface-hover bg-surface-secondary rounded-full w-8 h-8 flex items-center justify-center transition-all"
						>
							<X class="text-tertiary" />
						</button>
					</div>
				</div>
				<ContentSearchInner {search} bind:this={contentSearchInner}>
					<input
						slot="input-slot"
						bind:this={inputElement}
						placeholder="Search in the content of resources, scripts, flows and apps"
						bind:value={search}
						class="bg-surface !h-10 !px-4 !pr-10 !rounded-lg text-sm focus:outline-none"
					/>
				</ContentSearchInner>
			</div>
		</div>
	</Portal>
{/if}
