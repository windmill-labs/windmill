<script lang="ts">
	import { FileUp, Trash } from 'lucide-svelte'
	import Button from '../../common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ReadFileAs } from './model'
	import { sendUserToast } from '$lib/toast'

	type ConvertedFile = string | ArrayBuffer | null

	type FileWithPath = File & { path?: string }

	interface Props {
		class?: string
		style?: string
		accept?: string
		multiple?: boolean
		convertTo?: ReadFileAs | undefined
		hideIcon?: boolean
		iconSize?: number
		returnFileNames?: boolean
		submittedText?: string | undefined
		defaultFile?: string | string[] | undefined
		disabled?: boolean | undefined
		folderOnly?: boolean
		files?: FileWithPath[] | undefined
		selected_title?: import('svelte').Snippet
		children?: import('svelte').Snippet
		onChange?: (detail: any) => void
	}

	let {
		class: c = '',
		style = '',
		accept = '*',
		multiple = false,
		convertTo = undefined,
		hideIcon = false,
		iconSize = 24,
		returnFileNames = false,
		submittedText = undefined,
		defaultFile = undefined,
		disabled = undefined,
		folderOnly = false,
		files = $bindable(undefined),
		selected_title,
		children,
		onChange
	}: Props = $props()

	let input: HTMLInputElement

	let pointerStartX = 0
	let pointerStartY = 0

	function handlePointerDown(e: PointerEvent) {
		pointerStartX = e.clientX
		pointerStartY = e.clientY
	}

	async function handleFileChange(fileList: FileWithPath[] | null) {
		if (!fileList || !fileList.length) {
			files = undefined
			onChange?.(files)
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
		const filesArr: FileWithPath[] = []
		const dirReader = dirEntry.createReader()

		async function readEntries() {
			return new Promise<FileWithPath[]>((resolve) => {
				dirReader.readEntries(async (entries) => {
					if (entries.length === 0) {
						resolve(filesArr)
						return
					}

					const filePromises = entries.map(async (entry) => {
						return traverseFileTree(entry, path + dirEntry.name + '/')
					})
					const nestedFiles = await Promise.all(filePromises)
					filesArr.push(...nestedFiles.flat())
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
					const droppedFiles = await traverseFileTree(item, '')
					handleFileChange(droppedFiles)
				}
			} else {
				if (event.dataTransfer.files && event.dataTransfer.files.length) {
					if (!multiple && event.dataTransfer.files.length > 1) {
						sendUserToast('Only one file can be uploaded at a time')
						return
					} else {
						handleFileChange(Array.from(event.dataTransfer.files))
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
			onChange?.(converted)
		} else {
			onChange?.(files)
		}
	}

	export function clearFiles() {
		files = undefined
		dispatchChange()
	}
</script>

<button
	class={twMerge(
		`relative center-center flex flex-col gap-x-2 gap-y-1 justify-center items-center flex-wrap text-center font-normal text-hint text-xs rounded-md
		bg-surface-secondary
		border border-dashed border-nord-400 dark:border-nord-300 hover:border-nord-900 dark:hover:border-nord-900
		focus-within:border-blue-300 hover:bg-blue-50 dark:hover:bg-frost-900
		duration-200 px-1 py-8`,
		c
	)}
	ondragover={handleDragOver}
	ondrop={handleDrop}
	onpointerdown={handlePointerDown}
	onclick={(e) => {
		const deltaX = Math.abs(e.clientX - pointerStartX)
		const deltaY = Math.abs(e.clientY - pointerStartY)
		if (deltaX > 5 || deltaY > 5) {
			e.preventDefault()
			e.stopPropagation()
			return
		}
	}}
	{style}
	{disabled}
>
	{#if !hideIcon && !files}
		<FileUp size={iconSize} />
	{/if}
	{#if files}
		<div class="w-full max-h-full overflow-auto px-6">
			{#if selected_title}
				{@render selected_title()}
			{:else}
				<div class="text-center mb-2 px-2">
					{submittedText ? submittedText : `Selected file${files.length > 1 ? 's' : ''}`}:
				</div>
			{/if}
			<ul class="relative z-20 max-w-[500px] bg-surface rounded-lg overflow-hidden mx-auto">
				{#each files as { name }, i}
					<li
						class="flex justify-between items-center font-normal text-sm
					hover:bg-gray-300/20 duration-200 py-1 px-2 cursor-default"
					>
						<span class="pr-2 ellipsize">{name}</span>
						<Button
							size="xs"
							variant="default"
							iconOnly
							btnClasses="bg-transparent"
							startIcon={{ icon: Trash }}
							onclick={() => removeFile(i)}
							destructive
						/>
					</li>
				{/each}
			</ul>
		</div>
	{:else}
		{#if children}
			{@render children()}
		{:else}
			<span>Drag and drop {folderOnly ? 'a folder' : multiple ? 'files' : 'a file'}</span>
		{/if}
	{/if}
	<input
		class="!absolute !inset-0 !z-10 !opacity-0 !cursor-pointer"
		type="file"
		{...{ webkitdirectory: folderOnly }}
		title={files ? `${files.length} file${files.length > 1 ? 's' : ''} chosen` : 'No file chosen'}
		bind:this={input}
		onchange={({ currentTarget }) => {
			handleFileChange(currentTarget.files ? Array.from(currentTarget.files) : null)
		}}
		{accept}
		{multiple}
	/>
	{#if defaultFile && (!Array.isArray(defaultFile) || defaultFile.length > 0)}
		<div class="w-full border-dashed border-t-2 text-2xs pt-1 text-primary mt-2">
			Default file: <span class="text-nord-900">{defaultFile}</span>
		</div>
	{/if}
</button>
