<script lang="ts">
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { FileUp, Loader2 } from 'lucide-svelte'

	export let title: string
	export let open: boolean = false
	export let progressPct: number | undefined = undefined
	export let errorMsg: string | undefined = undefined

	export let fileKey: string | undefined = undefined
	export let fileToUpload: File | undefined = undefined

	const dispatch = createEventDispatcher()

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}
</script>

{#if open}
	<div
		transition:fadeFast|local
		class={'absolute top-0 bottom-0 left-0 right-0 z-[5000]'}
		role="dialog"
	>
		<div
			class={classNames(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
			)}
		/>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class={classNames(
						'relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
						open
							? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
							: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
					)}
				>
					<div class="flex flex-col gap-2">
						<h3 class="text-lg font-medium text-primary">
							{title}
						</h3>
						<div class="flex items-center gap-2">
							<span>Key: </span>
							<input
								type="text"
								placeholder="folder/nested/file.txt"
								bind:value={fileKey}
								class="text-2xl grow"
							/>
						</div>

						<div class="w-full h-full">
							<input
								type="file"
								title={fileToUpload ? `${fileToUpload.name}` : 'No file chosen'}
								on:change={({ currentTarget }) => {
									if (
										currentTarget.files === undefined ||
										currentTarget.files === null ||
										currentTarget.files.length === 0
									) {
										fileToUpload = undefined
									} else {
										fileToUpload = currentTarget.files[0]
										if (fileKey === undefined || fileKey === '') {
											fileKey = fileToUpload.name
										}
									}
								}}
								accept="*"
								multiple={false}
							/>
						</div>

						<div class="flex w-full bg-gray-200 rounded-full h-4 overflow-hidden">
							<div class="h-full bg-blue-400" style="width: {progressPct ?? 0}%" />
						</div>

						{#if errorMsg !== undefined}
							<div class="text-red-500 dark:text-red-400 text-sm">
								{errorMsg}
							</div>
						{/if}
					</div>
					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						<Button
							disabled={progressPct !== undefined}
							on:click={() => dispatch('confirmed')}
							color="blue"
							size="sm"
							startIcon={progressPct !== undefined
								? { icon: Loader2, classes: 'animate-spin' }
								: { icon: FileUp }}
						>
							<span>Upload</span>
						</Button>
						<Button on:click={() => dispatch('canceled')} color="light" size="sm">
							<span>Cancel</span>
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
