<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { FileUp, Trash } from 'lucide-svelte'
	import Button from '../../common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ReadFileAs } from './model'
	import { sendUserToast } from '$lib/toast'

	type ConvertedFile = string | ArrayBuffer | null

	let c = ''
	export { c as class }
	export let style = ''
	export let accept = '*'
	export let multiple = false
	export let convertTo: ReadFileAs | undefined = undefined
	export let hideIcon = false
	export let iconSize = 36
	export let returnFileNames = false
	export let submittedText: string | undefined = undefined
	export let defaultFile: string | string[] | undefined = undefined
	export let disabled: boolean | undefined = undefined
	export let folderOnly = false

	const dispatch = createEventDispatcher()
	let input: HTMLInputElement
	type FileWithPath = File & { path?: string }
	export let files: FileWithPath[] | undefined = undefined

	async function onChange(fileList: FileWithPath[] | null) {
		if (!fileList || !fileList.length) {
			files = undefined
			dispatch('change', files)
			return
		}

		if (multiple && files) {
			files = [...files, ...fileList]
		} else {
			files = fileList
		}

		// Needs to be reset so the same file can be selected
		// multiple times in a row
		input.value = ''

		dispatchChange()
	}

	async function convertFile(file: File): Promise<string | ArrayBuffer | null> {
		return new Promise((resolve) => {
			if (!convertTo) {
				return resolve(null)
			}
			const reader = new FileReader()
			reader.onloadend = () => {
				resolve(reader.result)
			}
			switch (convertTo) {
				case 'buffer':
					reader.readAsArrayBuffer(file)
					break
				case 'binary':
					reader.readAsBinaryString(file)
					break
				case 'base64':
					reader.readAsDataURL(file)
					break
				case 'text':
					reader.readAsText(file)
					break
			}
		})
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault()
		if (event.dataTransfer) {
			event.dataTransfer.dropEffect = 'copy'
		}
	}

	async function handleFile(fileEntry: FileSystemFileEntry): Promise<FileWithPath> {
		return new Promise((resolve, reject) => {
			fileEntry.file(resolve, reject)
		})
	}

	async function handleDirectory(
		dirEntry: FileSystemDirectoryEntry,
		path: string
	): Promise<FileWithPath[]> {
		const files: FileWithPath[] = []
		const dirReader = dirEntry.createReader()

		async function readEntries() {
			return new Promise<FileWithPath[]>((resolve) => {
				dirReader.readEntries(async (entries) => {
					if (entries.length === 0) {
						resolve(files)
						return
					}

					const filePromises = entries.map(async (entry) => {
						return traverseFileTree(entry, path + dirEntry.name + '/')
					})
					const nestedFiles = await Promise.all(filePromises)
					files.push(...nestedFiles.flat())
					// readEntries only return up to 100 files
					// continue reading if more files exist
					resolve(await readEntries())
				})
			})
		}

		return readEntries()
	}

	async function traverseFileTree(entry: FileSystemEntry, path = ''): Promise<FileWithPath[]> {
		if (entry.isFile) {
			const file = await handleFile(entry as FileSystemFileEntry)
			file.path = path + file.name
			return [file]
		} else if (entry.isDirectory) {
			return handleDirectory(entry as FileSystemDirectoryEntry, path)
		}
		return []
	}

	async function handleDrop(event: DragEvent) {
		event.preventDefault()
		if (event.dataTransfer) {
			if (folderOnly) {
				const item = event.dataTransfer.items[0]?.webkitGetAsEntry()
				if (item) {
					const files = await traverseFileTree(item, '')
					onChange(files)
				}
			} else {
				if (event.dataTransfer.files && event.dataTransfer.files.length) {
					if (!multiple && event.dataTransfer.files.length > 1) {
						sendUserToast('Only one file can be uploaded at a time')
						return
					} else {
						onChange(Array.from(event.dataTransfer.files))
					}
				}
			}
		}
	}

	function removeFile(index: number) {
		if (!files) return
		files.splice(index, 1)
		files = files.length ? files : undefined
		dispatchChange()
	}

	async function dispatchChange() {
		files = files
		if (convertTo && files) {
			const promises = files.map(convertFile)
			let converted: ConvertedFile[] | { name: string; data: ConvertedFile }[] =
				await Promise.all(promises)
			if (returnFileNames) {
				converted = converted.map((c, i) => ({ name: files![i].name, data: c }))
			}
			dispatch('change', converted)
		} else {
			dispatch('change', files)
		}
	}

	export function clearFiles() {
		files = undefined
		dispatchChange()
	}
</script>

<button
	class={twMerge(
		`relative center-center flex-col text-center font-medium text-tertiary 
		border border-dashed border-gray-400 hover:border-blue-500 
		focus-within:border-blue-300 hover:bg-blue-50 dark:hover:bg-frost-900  
		duration-200 rounded-component p-1`,
		c
	)}
	on:dragover={handleDragOver}
	on:drop={handleDrop}
	{style}
	{disabled}
>
	{#if !hideIcon && !files}
		<FileUp size={iconSize} class="mb-2" />
	{/if}
	{#if files}
		<div class="w-full max-h-full overflow-auto px-6">
			<slot name="selected-title">
				<div class="text-center mb-2 px-2">
					{submittedText ? submittedText : `Selected file${files.length > 1 ? 's' : ''}`}:
				</div>
			</slot>
			<ul class="relative z-20 max-w-[500px] bg-surface rounded-lg overflow-hidden mx-auto">
				{#each files as { name }, i}
					<li
						class="flex justify-between items-center font-normal text-sm
					hover:bg-gray-300/20 duration-200 py-1 px-2 cursor-default"
					>
						<span class="pr-2 ellipsize">{name}</span>
						<Button
							size="xs"
							color="red"
							variant="border"
							iconOnly
							btnClasses="bg-transparent"
							startIcon={{ icon: Trash }}
							on:click={() => removeFile(i)}
						/>
					</li>
				{/each}
			</ul>
		</div>
	{:else}
		<slot>
			<span>Drag and drop {folderOnly ? 'a folder' : multiple ? 'files' : 'a file'}</span>
		</slot>
	{/if}
	<input
		class="!absolute !inset-0 !z-10 !opacity-0 !cursor-pointer"
		type="file"
		{...{ webkitdirectory: folderOnly }}
		title={files ? `${files.length} file${files.length > 1 ? 's' : ''} chosen` : 'No file chosen'}
		bind:this={input}
		on:change={({ currentTarget }) => {
			onChange(currentTarget.files ? Array.from(currentTarget.files) : null)
		}}
		{accept}
		{multiple}
		{...$$restProps}
	/>
	{#if defaultFile && (!Array.isArray(defaultFile) || defaultFile.length > 0)}
		<div class="w-full border-dashed border-t-2 text-2xs pt-1 text-tertiary mt-2">
			Default file: <span class="text-blue-500">{defaultFile}</span>
		</div>
	{/if}
</button>
