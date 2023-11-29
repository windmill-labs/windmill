<script lang="ts">
	import { File, FolderClosed, FolderOpen, RotateCw, Loader2 } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService } from '$lib/gen'
	import { displayDate, displaySize, emptyString } from '$lib/utils'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import { createEventDispatcher } from 'svelte'
	import VirtualList from 'svelte-tiny-virtual-list'
	import TableSimple from './TableSimple.svelte'

	let workspaceSettingsInitialized = true

	export let readOnlyMode: boolean

	export let initialFileKey: { s3: string }
	let initialFileKeyInternalCopy: { s3: string }
	export let selectedFileKey: { s3: string } | undefined = undefined

	let csvSeparatorChar: string = ','

	let dispatch = createEventDispatcher()

	let drawer: Drawer
	let allFilesByKey: Record<
		string,
		{
			type: 'folder' | 'leaf'
			full_key: string
			display_name: string
			collapsed: boolean
			parentPath: string | undefined
			nestingLevel: number
		}
	> = {}
	let displayedFileKeys: string[] = []
	let paginationMarker: string | undefined = undefined

	let listDivHeight: number = 0

	let fileMetadata:
		| {
				fileKey: string
				mimeType: string | undefined
				size: number | undefined
				sizeStr: string | undefined
				lastModified: string | undefined
		  }
		| undefined = undefined
	let filePreviewLoading: boolean = false
	let filePreview:
		| {
				fileKey: string
				contentPreview: string | undefined
				contentType: string | undefined
		  }
		| undefined = undefined

	async function loadFiles() {
		let availableFiles = await HelpersService.listStoredFiles({
			workspace: $workspaceStore!,
			maxKeys: 1000, // fixed pages of 1000 files for now
			marker: paginationMarker
		})

		for (let file_path of availableFiles.windmill_large_files) {
			let split_path = file_path.s3.split('/')
			let parent_path: string | undefined = undefined
			let current_path: string | undefined = undefined
			let nestingLevel = 0
			for (let i = 0; i < split_path.length; i++) {
				parent_path = current_path
				current_path = current_path === undefined ? split_path[i] : current_path + split_path[i]

				if (i < split_path.length - 1) {
					current_path += '/'
				}

				nestingLevel = i * 2
				if (allFilesByKey[current_path] !== undefined) {
					continue
				}
				allFilesByKey[current_path] = {
					type: i === split_path.length - 1 ? 'leaf' : 'folder',
					full_key: current_path,
					display_name: split_path[i],
					collapsed: true, // folders collapsed by default
					parentPath: parent_path,
					nestingLevel: nestingLevel
				}
				if (i == 0) {
					displayedFileKeys.push(current_path)
				}
			}
		}
		displayedFileKeys = displayedFileKeys.sort()
		if (availableFiles.next_marker !== undefined) {
			paginationMarker = availableFiles.next_marker
		}

		// before returning, un-collapse the folders containing the selected file (if any)
		if (selectedFileKey !== undefined && !emptyString(selectedFileKey.s3)) {
			let split_path = selectedFileKey.s3.split('/')
			let current_path: string | undefined = undefined
			for (let i = 0; i < split_path.length; i++) {
				current_path = current_path === undefined ? split_path[i] : current_path + split_path[i]
				if (i < split_path.length - 1) {
					current_path += '/'
				}
				let indexOf = displayedFileKeys.indexOf(current_path)
				if (indexOf >= 0) {
					selectItem(indexOf, true)
				}
			}
		}
	}

	async function loadFileMetadataPlusPreviewAsync(fileKey: string | undefined) {
		if (fileKey === undefined) {
			return
		}
		let fileMetadataRaw = await HelpersService.loadFileMetadata({
			workspace: $workspaceStore!,
			fileKey: fileKey
		})

		if (fileMetadataRaw !== undefined) {
			fileMetadata = {
				fileKey: fileKey,
				size: fileMetadataRaw.size_in_bytes,
				sizeStr: displaySize(fileMetadataRaw.size_in_bytes),
				mimeType: fileMetadataRaw.mime_type,
				lastModified: displayDate(fileMetadataRaw.last_modified)
			}
		}
		// async call
		loadFilePreview(fileKey, fileMetadataRaw.size_in_bytes, fileMetadataRaw.mime_type)
	}

	async function loadFilePreview(fileKey: string, fileSizeInBytes?: number, fileMimeType?: string) {
		filePreviewLoading = true
		let filePreviewRaw = await HelpersService.loadFilePreview({
			workspace: $workspaceStore!,
			fileKey: fileKey,
			fileSizeInBytes: fileSizeInBytes,
			fileMimeType: fileMimeType,
			csvSeparator: csvSeparatorChar,
			readBytesFrom: 0,
			readBytesLength: 1 * 1024 * 1024 // For now static limit of 1Mb per file
		})

		if (filePreviewRaw !== undefined) {
			filePreview = {
				fileKey: fileKey,
				contentPreview: filePreviewRaw.content,
				contentType: filePreviewRaw.content_type
			}
		}
		filePreviewLoading = false
	}

	export async function open() {
		reloadContent()
		drawer.openDrawer?.()
	}

	async function reloadContent() {
		initialFileKeyInternalCopy = { ...initialFileKey }
		try {
			await HelpersService.datasetStorageTestConnection({ workspace: $workspaceStore! })
		} catch (e) {
			console.error('Workspace not connected to S3 bucket: ', e)
			workspaceSettingsInitialized = false
			return
		}
		await loadFiles() // TODO: Potentially load only on the first open and add a refresh button
		if (selectedFileKey !== undefined) {
			if (allFilesByKey[selectedFileKey.s3] === undefined) {
				selectedFileKey = { s3: '' }
			} else {
				loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
			}
		}
	}

	async function selectAndClose() {
		drawer.closeDrawer?.()
	}

	async function exit() {
		if (initialFileKeyInternalCopy !== undefined) {
			selectedFileKey = { ...initialFileKeyInternalCopy }
		}
		drawer.closeDrawer?.()
	}

	function selectItem(index: number, toggleCollapsed: boolean = true) {
		let item_key = displayedFileKeys[index]
		let item = allFilesByKey[item_key]
		if (item.type === 'folder') {
			if (toggleCollapsed) {
				item.collapsed = !item.collapsed
			}
			if (item.collapsed) {
				// Remove the element nested in that folder from displayed_file_keys
				let elt_to_remove = 0
				for (let i = index + 1; i < displayedFileKeys.length; i++) {
					let file_key = displayedFileKeys[i]
					if (file_key.startsWith(item_key)) {
						elt_to_remove += 1
					} else {
						break
					}
				}
				if (elt_to_remove > 0) {
					displayedFileKeys.splice(index + 1, elt_to_remove)
				}
			} else {
				// Re-add the currently hidden element to displayed_file_keys
				for (let file_key in allFilesByKey) {
					let file_info = allFilesByKey[file_key]
					if (file_info.parentPath === item_key) {
						displayedFileKeys.push(file_key)
						if (file_info.type === 'folder' && !file_info.collapsed) {
							selectItem(displayedFileKeys.length - 1, false)
						}
					}
				}
			}
			displayedFileKeys = displayedFileKeys.sort()
		} else {
			selectedFileKey = {
				s3: item_key
			}
			loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
		}
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<Drawer
	bind:this={drawer}
	on:close={() => {
		dispatch('close')
	}}
	size="1200px"
>
	<DrawerContent
		title="S3 file browser"
		overflow_y={false}
		on:close={exit}
		tooltip="Files present in the Workspace S3 bucket. You can set the workspace S3 bucket in the settings."
		documentationLink="https://www.windmill.dev/docs/integrations/s3"
	>
		{#if workspaceSettingsInitialized === false}
			<Alert type="error" title="Workspace not connected to any S3 storage">
				<div class="flex flex-row gap-x-1 w-full items-center">
					<p class="text-clip grow min-w-0">
						The workspace needs to be connected to an S3 storage to use this feature. You can <a
							target="_blank"
							href="/workspace_settings?tab=windmill_lfs">configure it here</a
						>.
					</p>
					<Button
						variant="border"
						color="light"
						on:click={reloadContent}
						startIcon={{ icon: RotateCw }}
					/>
				</div>
			</Alert>
		{:else}
			<div class="flex flex-row border rounded-md h-full" bind:clientHeight={listDivHeight}>
				<div class="min-w-[30%] border-r">
					{#if displayedFileKeys.length === 0}
						<div class="p-4 text-tertiary text-xs text-center italic">
							No files in the workspace S3 bucket
						</div>
					{:else}
						<VirtualList
							width="100%"
							height={listDivHeight}
							itemCount={displayedFileKeys.length}
							itemSize={42}
						>
							<div slot="item" let:index let:style {style} class="hover:bg-surface-hover border">
								{@const file_info = allFilesByKey[displayedFileKeys[index]]}
								<div
									on:click={() => selectItem(index)}
									class={`flex flex-row h-full font-semibold text-xs items-center justify-start ${
										selectedFileKey !== undefined && selectedFileKey.s3 === file_info.full_key
											? 'bg-surface-hover'
											: ''
									} `}
								>
									<div
										class={`flex flex-row w-full ml-${
											2 + file_info.nestingLevel
										} gap-2 h-full items-center`}
									>
										{#if file_info.type === 'folder'}
											{#if file_info.collapsed}<FolderClosed size={16} />{:else}<FolderOpen
													size={16}
												/>{/if}
											<div class="truncate text-ellipsis w-56">
												{file_info.display_name}
											</div>
										{:else}
											<File size={16} />
											<div class="truncate text-ellipsis w-56">
												{file_info.display_name}
											</div>
										{/if}
									</div>
								</div>
							</div>
							<div slot="footer">
								{#if !emptyString(paginationMarker)}
									<button
										class="text-secondary underline text-2xs whitespace-nowrap text-center w-full"
										on:click={loadFiles}
										>More files in bucket. Click here to load more...
									</button>
								{/if}
							</div>
						</VirtualList>
					{/if}
				</div>
				<div class="flex flex-col h-full w-full overflow-auto">
					{#if fileMetadata === undefined}
						<div class="p-4">
							<Section label="Select a file for preview" />
						</div>
					{:else}
						<div class="p-4 gap-2">
							<Section label={fileMetadata.fileKey} />
							<TableSimple
								headers={['Last modified', 'Size', 'Type']}
								data={[fileMetadata]}
								keys={['lastModified', 'sizeStr', 'mimeType']}
							/>
						</div>
					{/if}

					<div class="flex flex-col h-full w-full overflow-auto text-xs p-4 bg-surface-secondary">
						{#if fileMetadata !== undefined && filePreview !== undefined}
							<div class="flex h-6 items-center text-tertiary mb-4">
								{#if filePreview.contentType === 'Unknown'}
									Type of file not supported for preview
								{:else if filePreview.contentType === 'Csv'}
									Previewing a {filePreview.contentType?.toLowerCase()} file. Change the separator:
									<div class="inline-flex w-12 ml-2">
										<select
											class="h-8"
											bind:value={csvSeparatorChar}
											on:change={(e) =>
												loadFilePreview(
													fileMetadata?.fileKey ?? '',
													fileMetadata?.size,
													fileMetadata?.mimeType
												)}
										>
											<option value=",">,</option>
											<option value=";">;</option>
											<option value="|">|</option>
										</select>
									</div>
								{:else}
									Previewing a {filePreview.contentType?.toLowerCase()} file
								{/if}
							</div>
							<pre class="grow whitespace-no-wrap break-words"
								>{#if !emptyString(filePreview.contentPreview)}{filePreview.contentPreview}{:else if filePreview.contentType !== undefined}Preview impossible.{/if}</pre
							>
						{:else if filePreviewLoading}
							<div class="flex h-6 items-center text-tertiary mb-4">
								<Loader2 size={12} class="animate-spin mr-1" /> File preview loading
							</div>
						{/if}
					</div>
				</div>
			</div>
		{/if}

		<div slot="actions" class="flex gap-1">
			{#if !readOnlyMode}
				<Button
					disable={selectedFileKey === undefined || emptyString(selectedFileKey.s3)}
					on:click={selectAndClose}>Select</Button
				>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
