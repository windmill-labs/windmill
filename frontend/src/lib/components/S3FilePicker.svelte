<script lang="ts">
	import {
		File as FileIcon,
		FolderClosed,
		FolderOpen,
		RotateCw,
		Loader2,
		Download,
		Trash,
		FileUp,
		MoveRight
	} from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService } from '$lib/gen'
	import { displayDate, displaySize, emptyString, sendUserToast } from '$lib/utils'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import { createEventDispatcher } from 'svelte'
	import VirtualList from 'svelte-tiny-virtual-list'
	import TableSimple from './TableSimple.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import FileUploadModal from './common/fileUpload/FileUploadModal.svelte'

	let deletionModalOpen = false
	let fileDeletionInProgress = false

	let fileListUnavailable: boolean | undefined = undefined

	let moveModalOpen = false
	let moveDestKey: string | undefined = undefined
	let fileMoveInProgress = false

	let uploadModalOpen = false

	let workspaceSettingsInitialized = true

	export let fromWorkspaceSettings: boolean = false
	export let readOnlyMode: boolean

	export let initialFileKey: { s3: string } | undefined = undefined
	let initialFileKeyInternalCopy: { s3: string }
	export let selectedFileKey: { s3: string } | undefined = undefined

	let csvSeparatorChar: string = ','
	let csvHasHeader: boolean = true

	let dispatch = createEventDispatcher()

	let drawer: Drawer

	let fileInfoLoading: boolean = true
	let fileListLoading: boolean = true
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

	let listMarkers: string[]
	let page = 0

	const maxKeys = 1000

	let count = 0

	let filter = ''

	let timeout: NodeJS.Timeout | undefined = undefined
	let firstLoad = true
	$: filter != undefined && onFilterChange()

	function onFilterChange() {
		if (!firstLoad) {
			timeout && clearTimeout(timeout)
			timeout = setTimeout(() => {
				page = 0
				listMarkers = []
				loadFiles()
			}, 500)
		} else {
			firstLoad = false
		}
	}

	async function loadFiles() {
		fileListLoading = true
		let availableFiles = await HelpersService.listStoredFiles({
			workspace: $workspaceStore!,
			maxKeys: maxKeys, // fixed pages of 1000 files for now
			marker: page == 0 ? undefined : listMarkers[page - 1],
			prefix: filter.trim() != '' ? filter : undefined
		})
		if (
			availableFiles.restricted_access === null ||
			availableFiles.restricted_access === undefined ||
			availableFiles.restricted_access === true
		) {
			fileListUnavailable = true
			loadFileMetadataPlusPreviewAsync(selectedFileKey?.s3)
			return
		}
		fileListUnavailable = false
		allFilesByKey = {}
		displayedFileKeys = []
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
		if (listMarkers.length == page) {
			count = availableFiles.windmill_large_files.length
			const nextMarker =
				availableFiles.windmill_large_files?.[availableFiles.windmill_large_files.length - 1]?.s3
			if (nextMarker) listMarkers.push(nextMarker)
		}
		displayedFileKeys = displayedFileKeys.sort()

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
		fileListLoading = false
		fileInfoLoading = false
	}

	async function loadFileMetadataPlusPreviewAsync(fileKey: string | undefined) {
		if (fileKey === undefined || emptyString(fileKey)) {
			fileInfoLoading = false
			return
		}
		fileInfoLoading = true
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
			csvHasHeader: csvHasHeader,
			readBytesFrom: 0,
			readBytesLength: 128 * 1024 // For now static limit of 128Kb per file
		})

		let filePreviewContent = filePreviewRaw.content
		if (
			filePreviewContent !== null &&
			filePreviewContent !== undefined &&
			filePreviewContent.length >= 128 * 1024
		) {
			filePreviewContent =
				filePreviewContent?.substring(0, 128 * 1024 - 35) +
				'\n\n ... FILE CONTENT TRUNCATED ...\n\n'
		}

		if (filePreviewRaw !== undefined) {
			filePreview = {
				fileKey: fileKey,
				contentPreview: filePreviewContent,
				contentType: filePreviewRaw.content_type
			}
		}
		filePreviewLoading = false
		fileInfoLoading = false
	}

	async function deleteFileFromS3(fileKey: string | undefined) {
		fileDeletionInProgress = true
		if (fileKey === undefined) {
			return
		}
		try {
			await HelpersService.deleteS3File({
				workspace: $workspaceStore!,
				fileKey: fileKey
			})
		} finally {
			fileDeletionInProgress = false
			deletionModalOpen = false
		}
		sendUserToast(`${fileKey} deleted from S3 bucket`)
		selectedFileKey = { s3: '' }
		const idx = displayedFileKeys.indexOf(fileKey)
		if (idx >= 0) {
			displayedFileKeys.splice(idx, 1)
			displayedFileKeys = [...displayedFileKeys]
		}
		delete allFilesByKey[fileKey]
	}

	async function moveS3File(srcFileKey: string | undefined, destFileKey: string | undefined) {
		fileMoveInProgress = true
		if (srcFileKey === undefined || emptyString(destFileKey)) {
			return
		}
		try {
			await HelpersService.moveS3File({
				workspace: $workspaceStore!,
				srcFileKey: srcFileKey,
				destFileKey: destFileKey!
			})
		} finally {
			fileMoveInProgress = false
			moveModalOpen = false
		}
		sendUserToast(`${srcFileKey} moved to ${destFileKey}`)
		selectedFileKey = { s3: destFileKey! }
		await loadFiles()
		await loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
	}

	export async function open(preSelectedFileKey: { s3: string } | undefined = undefined) {
		if (preSelectedFileKey !== undefined) {
			initialFileKey = { ...preSelectedFileKey }
			selectedFileKey = { ...preSelectedFileKey }
		}
		displayedFileKeys = []
		allFilesByKey = {}
		count = 0
		page = 0
		filter = ''
		listMarkers = []
		fileMetadata = undefined
		filePreview = undefined
		reloadContent()
		drawer.openDrawer?.()
	}

	async function reloadContent() {
		if (initialFileKey !== undefined) {
			initialFileKeyInternalCopy = { ...initialFileKey }
		}
		try {
			await HelpersService.datasetStorageTestConnection({ workspace: $workspaceStore! })
			workspaceSettingsInitialized = true
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
			{#if fromWorkspaceSettings}
				<Alert type="error" title="Connection to remote S3 bucket unsuccessful">
					<div class="flex flex-row gap-x-1 w-full items-center">
						<p class="text-clip grow min-w-0">
							Double check the S3 resource fields and try again.
						</p>
					</div>
				</Alert>
			{:else}
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
			{/if}
		{:else}
			{#if fileListUnavailable == true}
				<div class="mb-2">
					<Alert type="info" title="Access to S3 bucket restricted">
						<p>
							You don't have access to the S3 bucket resource and your administrator has restricted
							the access to it. You are not authorized to browse the bucket content. If you think
							this is incorrect, please contact your workspace administrator.
						</p>
						<p>
							More info in <a
								href="https://www.windmill.dev/docs/core_concepts/persistent_storage#connect-your-windmill-workspace-to-your-s3-bucket"
								target="_blank">Windmill's documentation</a
							></p
						></Alert
					>
				</div>
			{/if}
			<div class="flex flex-row border rounded-md h-full">
				{#if !fileListUnavailable}
					<div class="min-w-[30%] border-r h-full flex flex-col">
						<div class="w-12/12 pb-2 flex flex-row mb-1 gap-1">
							<input type="text" placeholder="Folder prefix" bind:value={filter} class="text-2xl" />
						</div>
						{#if fileListLoading === false && displayedFileKeys.length === 0}
							<div class="p-4 text-tertiary text-xs text-center italic">
								No files in the workspace S3 bucket at that prefix
							</div>
						{:else}
							<div class="grow max-h-3/4" bind:clientHeight={listDivHeight}>
								<VirtualList
									width="100%"
									height={listDivHeight}
									itemCount={displayedFileKeys.length}
									itemSize={42}
								>
									<div
										slot="item"
										let:index
										let:style
										{style}
										class="hover:bg-surface-hover border"
									>
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
													<FileIcon size={16} />
													<div class="truncate text-ellipsis w-56">
														{file_info.display_name}
													</div>
												{/if}
											</div>
										</div>
									</div></VirtualList
								>
							</div>
							<div
								class="flex gap-2 text-2xs items-center text-secondary px-2 w-full h-max-[30px] pt-2 border-t"
							>
								<div>{count} items on this page</div>
								<div>Page {page + 1}</div>

								{#if count == maxKeys}
									<button
										class="text-secondary border p-1 underline text-2xs whitespace-nowrap text-center"
										on:click={() => {
											page -= 1
											loadFiles()
										}}
										>Previous
									</button>
									<button
										class="text-secondary border p-1 underline text-2xs whitespace-nowrap text-center"
										on:click={() => {
											page += 1
											loadFiles()
										}}
										>Next
									</button>
								{/if}
							</div>
							{#if fileListLoading === true}
								<div class="flex text-secondary mt-1 text-xs justify-center items-center w-full">
									<Loader2 size={12} class="animate-spin mr-1" /> Loading content
								</div>
							{/if}
						{/if}
					</div>
				{/if}
				<div class="flex flex-col h-full w-full overflow-auto">
					{#if fileMetadata === undefined}
						<div class="p-4">
							{#if fileInfoLoading}
								<Section label="Loading..." />
							{:else if fileListUnavailable}
								<Section label="No file to preview" />
							{:else}
								<Section label="Select a file to preview" />
							{/if}
						</div>
					{:else}
						<div class="p-4 gap-2">
							<Section label={fileMetadata.fileKey}>
								<div slot="action" class="flex gap-2">
									{#if filePreview !== undefined}
										<Button
											title="Download file from S3"
											variant="border"
											color="light"
											href={`/api/w/${$workspaceStore}/job_helpers/download_s3_file?file_key=${fileMetadata?.fileKey}`}
											download={fileMetadata?.fileKey.split('/').pop() ?? 'unnamed_download.file'}
											startIcon={{ icon: Download }}
											iconOnly={true}
										/>
										<Button
											title="Move file"
											variant="border"
											color="light"
											on:click={() => {
												moveDestKey = fileMetadata?.fileKey ?? ''
												moveModalOpen = true
											}}
											startIcon={{ icon: MoveRight }}
											iconOnly={true}
										/>
										<Button
											title="Delete file from S3"
											variant="border"
											color="red"
											on:click={() => {
												deletionModalOpen = true
											}}
											startIcon={{ icon: Trash }}
											iconOnly={true}
										/>
									{/if}
								</div>
							</Section>
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
									Type of file not supported for preview.
								{:else if filePreview.contentType === 'Csv'}
									Previewing a {filePreview.contentType?.toLowerCase()} file. Separator character:
									<div class="inline-flex w-12 ml-2 mr-2">
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
											<option value="\t">\t</option>
											<option value="|">|</option>
										</select>
									</div>
									Header row:
									<div class="inline-flex item-center w-4 ml-2 mr-2">
										<input
											on:focus
											on:click
											disabled={false}
											type="checkbox"
											id="csv-header"
											class="h-5"
											bind:checked={csvHasHeader}
											on:change|stopPropagation={(e) =>
												loadFilePreview(
													fileMetadata?.fileKey ?? '',
													fileMetadata?.size,
													fileMetadata?.mimeType
												)}
										/>
									</div>
								{:else}
									Previewing a {filePreview.contentType?.toLowerCase()} file.
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
					variant="border"
					color="light"
					disabled={workspaceSettingsInitialized === false}
					startIcon={{ icon: FileUp }}
					on:click={() => {
						uploadModalOpen = true
					}}>Upload File</Button
				>
				{#if !fromWorkspaceSettings}
					<Button
						disable={selectedFileKey === undefined || emptyString(selectedFileKey.s3)}
						on:click={selectAndClose}>Select</Button
					>
				{/if}
			{/if}
		</div>
	</DrawerContent>
</Drawer>

<ConfirmationModal
	open={deletionModalOpen}
	title="Permanently delete file"
	confirmationText="Delete permanently"
	on:canceled={() => {
		deletionModalOpen = false
	}}
	on:confirmed={() => {
		deleteFileFromS3(fileMetadata?.fileKey)
	}}
	keyListen={false}
	bind:loading={fileDeletionInProgress}
>
	<div class="flex flex-col w-full space-y-4">
		<span
			>Are you sure you want to permanently delete {fileMetadata?.fileKey} from the S3 bucket?</span
		>
	</div>
</ConfirmationModal>

<ConfirmationModal
	open={moveModalOpen}
	title="Move file to new location"
	confirmationText="Move"
	on:canceled={() => {
		moveModalOpen = false
	}}
	on:confirmed={() => {
		moveS3File(fileMetadata?.fileKey, moveDestKey)
	}}
	keyListen={false}
	bind:loading={fileMoveInProgress}
>
	<div class="flex flex-col space-y-4">
		<div class="flex items-center justify-between">
			<span class="w-24">New key: </span>
			<input
				type="text"
				placeholder="folder/nested/file.txt"
				bind:value={moveDestKey}
				class="text-2xl"
			/>
		</div>
		<span>Are you sure you want to permanently move {fileMetadata?.fileKey}?</span>
	</div>
</ConfirmationModal>

<FileUploadModal
	open={uploadModalOpen}
	title="Upload file to S3 bucket"
	on:close={async (evt) => {
		uploadModalOpen = false
		if (evt.detail !== undefined && evt.detail !== null) {
			selectedFileKey = { s3: evt.detail }
			loadFiles()
			loadFileMetadataPlusPreviewAsync(evt.detail)
		}
	}}
/>
