<script lang="ts">
	import { classNames, emptyString } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { X } from 'lucide-svelte'
	import FileUpload from './FileUpload.svelte'

	export let title: string
	export let open: boolean = false

	export let fileKey: string | undefined = undefined

	let s3Folder: string = ''
	const dispatch = createEventDispatcher()

	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}

	function cleanFilePath(rawFolder: string, fileName: string) {
		if (emptyString(rawFolder)) {
			return fileName
		}
		if (!rawFolder.endsWith('/')) {
			rawFolder = `${rawFolder}/`
		}
		return `${rawFolder}${fileName}`
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
		></div>

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
						<div class="flex justify-between">
							<h3 class="text-lg font-medium text-primary">
								{title}
							</h3>
							<Button
								on:click={() => dispatch('close', fileKey)}
								title="Close"
								color="light"
								size="sm"
								iconOnly={true}
								startIcon={{ icon: X }}
							/>
						</div>
						<div class="flex items-center gap-2">
							<span>Folder: </span>
							<input
								type="text"
								placeholder="folder/nested/"
								bind:value={s3Folder}
								class="text-2xl grow"
							/>
						</div>

						<div class="w-full h-full">
							<FileUpload
								allowMultiple={true}
								pathTransformer={(file) => cleanFilePath(s3Folder, file.file.name)}
								on:addition={(evt) => {
									fileKey = evt.detail?.path
								}}
								on:deletion={(evt) => {
									fileKey = undefined
								}}
							/>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
