<script lang="ts">
	import {
		File as FileIcon,
		FolderClosed,
		FolderOpen,
		ChevronDown,
		RotateCw,
		Loader2,
		Download,
		Trash,
		MoveRight
	} from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		CancelablePromise,
		HelpersService,
		type DatasetStorageTestConnectionData,
		type DatasetStorageTestConnectionResponse,
		type DeleteS3FileData,
		type DeleteS3FileResponse,
		type ListStoredFilesData,
		type ListStoredFilesPagedData,
		type ListStoredFilesPagedResponse,
		type ListStoredFilesResponse,
		type LoadFileMetadataData,
		type LoadFileMetadataResponse,
		type LoadFilePreviewData,
		type LoadFilePreviewResponse,
		type MoveS3FileData,
		type MoveS3FileResponse
	} from '$lib/gen'
	import { base } from '$lib/base'
	import {
		displayDate,
		displaySize,
		emptyString,
		parseS3Object,
		sendUserToast,
		type S3Object
	} from '$lib/utils'
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'
	import { Alert, Button } from './common'
	import Section from './Section.svelte'
	import { createEventDispatcher, untrack, type Snippet } from 'svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import TableSimple from './TableSimple.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import FileUploadModal from './common/fileUpload/FileUploadModal.svelte'
	import S3FilePreview from './S3FilePreview.svelte'
	import { twMerge } from 'tailwind-merge'

	let deletionModalOpen = $state(false)
	let fileDeletionInProgress = $state(false)

	let fileListUnavailable: boolean | undefined = $state(undefined)

	let moveModalOpen = $state(false)
	let moveDestKey: string | undefined = $state(undefined)
	let fileMoveInProgress = $state(false)

	let initialFileKeyInternalCopy: { s3: string; storage?: string }
	interface Props {
		fromWorkspaceSettings?: boolean
		readOnlyMode: boolean
		initialFileKey?: { s3: string; storage?: string } | undefined
		selectedFileKey?: { s3: string; storage?: string } | undefined
		folderOnly?: boolean
		regexFilter?: RegExp | undefined
		hideS3SpecificDetails?: boolean
		rootPath?: string
		/** Workspace to browse S3 storage in — defaults to the nav workspace. */
		workspace?: string | undefined
		workspaceSettingsInitialized?: boolean
		storage?: string | undefined
		/** Browse this object storage resource directly instead of the workspace storage. */
		s3ResourcePath?: string | undefined
		uploadModalOpen?: boolean
		allFilesByKey?: Record<
			string,
			{
				type: 'folder' | 'leaf'
				full_key: string
				display_name: string
				collapsed: boolean
				parentPath: string | undefined
				nestingLevel: number
				count: number
			}
		>
		allowDelete?: boolean
		replaceUnauthorizedWarning?: Snippet
		/**
		 * Expand one folder level at a time instead of listing every key up front.
		 * Callers that override `listStoredFilesRequest` with a listing that has no
		 * paged counterpart (e.g. git repo files) must turn this off.
		 */
		lazyFolders?: boolean
		listStoredFilesRequest?: (d: ListStoredFilesData) => CancelablePromise<ListStoredFilesResponse>
		listStoredFilesPagedRequest?: (
			d: ListStoredFilesPagedData
		) => CancelablePromise<ListStoredFilesPagedResponse>
		loadFilePreviewRequest?: (d: LoadFilePreviewData) => CancelablePromise<LoadFilePreviewResponse>
		loadFileMetadataRequest?: (
			d: LoadFileMetadataData
		) => CancelablePromise<LoadFileMetadataResponse>
		deleteS3FileRequest?: (d: DeleteS3FileData) => CancelablePromise<DeleteS3FileResponse>
		moveS3FileRequest?: (d: MoveS3FileData) => CancelablePromise<MoveS3FileResponse>
		testConnectionRequest?: (
			d: DatasetStorageTestConnectionData
		) => CancelablePromise<DatasetStorageTestConnectionResponse>
	}

	let {
		fromWorkspaceSettings = false,
		readOnlyMode,
		initialFileKey = $bindable(undefined),
		selectedFileKey = $bindable(undefined),
		folderOnly = false,
		regexFilter = undefined,
		hideS3SpecificDetails = false,
		rootPath: initialRootPath = '',
		workspace = undefined,
		workspaceSettingsInitialized = $bindable(true),
		storage = $bindable(undefined),
		s3ResourcePath = undefined,
		uploadModalOpen = $bindable(false),
		allFilesByKey = $bindable({}),
		allowDelete = false,
		replaceUnauthorizedWarning,
		lazyFolders = true,
		listStoredFilesRequest = HelpersService.listStoredFiles,
		listStoredFilesPagedRequest = HelpersService.listStoredFilesPaged,
		loadFilePreviewRequest = HelpersService.loadFilePreview,
		loadFileMetadataRequest = HelpersService.loadFileMetadata,
		deleteS3FileRequest = HelpersService.deleteS3File,
		moveS3FileRequest = HelpersService.moveS3File,
		testConnectionRequest = HelpersService.datasetStorageTestConnection
	}: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let rootPath = $state(initialRootPath)
	let rootPathNestingLevel = $derived(1 * (rootPath.split('/').length - 1))

	let csvSeparatorChar: string = $state(',')
	let csvHasHeader: boolean = $state(true)

	let dispatch = createEventDispatcher<{
		close: { s3: string; storage: string | undefined } | undefined
		selectAndClose: { s3: string; storage: string | undefined }
	}>()

	let fileInfoLoading: boolean = $state(true)
	let fileListLoading: boolean = $state(true)
	let displayedFileKeys: string[] = $state([])

	let listDivHeight: number = $state(0)

	let fileMetadata:
		| {
				fileKey: string
				mimeType: string | undefined
				size: number | undefined
				sizeStr: string | undefined
				lastModified: string | undefined
		  }
		| undefined = $state(undefined)
	// `filePreviewLoading` was previously templated; now that the visual
	// preview lives in <S3FilePreview/> the picker only uses filePreview to
	// gate the toolbar and refine fileMetadata.mimeType, so the standalone
	// loading flag is gone.
	let filePreview:
		| {
				fileKey: string
				contentPreview: string | undefined
				contentType: string | undefined
		  }
		| undefined = $state(undefined)

	/** Identifies the metadata request that currently owns the preview pane. */
	let metadataRequestId = 0

	/**
	 * Flat pagination cursor: `listMarkers[n]` is where page `n + 1` resumes, so `page`
	 * may only advance over a page that actually loaded. Running ahead of `listMarkers`
	 * sends no marker and silently replays the first page, and never recovers, because
	 * the `listMarkers.length == page` guard stops recording from then on.
	 */
	let listMarkers: string[]
	let page = $state(0)

	const maxKeys = 1000
	/** Entries fetched per folder level before a "Load more" row appears. */
	const pageSize = 500

	let count = $state(0)
	let displayedCount = $state(0)
	/** Flat (non-lazy) listing: whether the last page came back full. */
	let flatHasMore = $state(false)

	let filter = $state('')

	let timeout: number | undefined = undefined
	let firstLoad = true

	function onFilterChange() {
		if (!firstLoad) {
			timeout && clearTimeout(timeout)
			timeout = setTimeout(() => {
				clearAndLoadFiles({ keepFilter: true })
			}, 500)
		} else {
			firstLoad = false
		}
	}

	/**
	 * Marks the synthetic "Load more" row belonging to a folder. Appended to the
	 * folder's own prefix so the plain lexicographic sort that orders the tree also
	 * places the row last among that folder's children.
	 */
	const LOAD_MORE_SUFFIX = '￿'

	type FolderState = { nextPageToken?: string; loading: boolean; loaded: boolean }
	let folderState: Record<string, FolderState> = $state({})

	/** Per-level listing only makes sense while browsing; searching stays flat. */
	let lazyMode = $derived(lazyFolders && filter.trim() === '')

	function nestingLevelOf(key: string): number {
		const slashes = (key.match(/\//g) ?? []).length
		return (key.endsWith('/') ? slashes - 1 : slashes) * 2
	}

	function parentPathOf(key: string): string | undefined {
		const body = key.endsWith('/') ? key.slice(0, -1) : key
		const idx = body.lastIndexOf('/')
		return idx === -1 ? undefined : body.slice(0, idx + 1)
	}

	/** The parentPath value carried by entries sitting at the browsing root. */
	let rootParentPath = $derived(rootPath === '' ? undefined : rootPath)

	function addEntry(key: string, type: 'folder' | 'leaf', displayName: string) {
		if (allFilesByKey[key] !== undefined) return
		allFilesByKey[key] = {
			type,
			full_key: key,
			display_name: displayName,
			collapsed: true,
			parentPath: parentPathOf(key),
			nestingLevel: nestingLevelOf(key),
			count: 0
		}
	}

	/**
	 * A level shows only when every folder above it is expanded. The browsing root is
	 * spelled `rootPath` by `folderState` but `undefined` by an entry's `parentPath`
	 * (there is no parent entry), so both spellings must resolve here — otherwise the
	 * root's own "Load more" row is filtered out and the top level is stuck on one page.
	 */
	function isLevelVisible(prefix: string | undefined): boolean {
		if (prefix === rootParentPath || prefix === rootPath) return true
		if (prefix === undefined) return false
		const info = allFilesByKey[prefix]
		if (info === undefined || info.collapsed) return false
		return isLevelVisible(info.parentPath)
	}

	/**
	 * Every key whose ancestors are all expanded, in depth-first order — full keys
	 * sort that way because a child always starts with its parent's
	 * delimiter-terminated prefix.
	 */
	function computeVisibleKeys(): string[] {
		const visible: string[] = []
		for (const key in allFilesByKey) {
			if (!key.startsWith(rootPath)) continue
			if (isLevelVisible(allFilesByKey[key].parentPath)) visible.push(key)
		}
		for (const prefix in folderState) {
			if (folderState[prefix].nextPageToken && isLevelVisible(prefix)) {
				visible.push(prefix + LOAD_MORE_SUFFIX)
			}
		}
		return visible.sort()
	}

	function refreshDisplayed() {
		const visible = computeVisibleKeys()
		displayedFileKeys = visible
		displayedCount = visible.filter((k) => !k.endsWith(LOAD_MORE_SUFFIX)).length
	}

	/**
	 * In-flight request per level. Callers that await `loadFolderPage` need the data
	 * to be there when it resolves; returning early on a concurrent load would hand
	 * them a resolved promise and no entries.
	 */
	let inFlightFolderLoads: Record<string, Promise<void>> = {}

	/**
	 * Bumped when a single level is invalidated on its own (a delete refetches it from
	 * page one). Requests are keyed by prefix alone, so without this a pending "Load
	 * more" for that level would be joined by the refetch, which would then return
	 * believing page one had been fetched — leaving the level showing only its later
	 * pages until a full reload.
	 */
	let folderEpoch: Record<string, number> = {}

	/**
	 * Bumped whenever the listing is thrown away (storage switch, filter change,
	 * reload). A request started before the bump belongs to the previous listing, so
	 * its response must not repopulate the cleared state — otherwise switching
	 * storage mid-load leaves the previous bucket's entries on screen.
	 */
	let listingGeneration = 0

	async function loadFolderPage(prefix: string, append: boolean = false): Promise<void> {
		const generation = listingGeneration
		const epoch = folderEpoch[prefix] ?? 0
		const pending = inFlightFolderLoads[prefix]
		if (pending) {
			await pending
			// Only reuse the joined result if it belongs to the same listing *and* the
			// same epoch of this level.
			if (generation === listingGeneration && epoch === (folderEpoch[prefix] ?? 0) && !append)
				return
		}
		const run = loadFolderPageInner(prefix, append)
		inFlightFolderLoads[prefix] = run.catch(() => {})
		try {
			await run
		} finally {
			delete inFlightFolderLoads[prefix]
		}
	}

	async function loadFolderPageInner(prefix: string, append: boolean) {
		const generation = listingGeneration
		const epoch = folderEpoch[prefix] ?? 0
		/** Whether this response still belongs to the listing and level that asked for it. */
		const stillCurrent = () =>
			generation === listingGeneration && epoch === (folderEpoch[prefix] ?? 0)
		const current = folderState[prefix] ?? { loading: false, loaded: false }
		if (append && current.nextPageToken === undefined) return
		folderState[prefix] = { ...current, loading: true }
		try {
			const page = await listStoredFilesPagedRequest({
				workspace: ws!,
				prefix,
				maxKeys: pageSize,
				pageToken: append ? current.nextPageToken : undefined,
				storage,
				s3ResourcePath
			})
			// This listing, or just this level, has been thrown away since the request
			// went out. Writing the entries back would resurrect the previous storage's
			// contents, or reinstate a file that was just deleted.
			if (!stillCurrent()) return
			// Absent counts as restricted, matching `loadFlatFiles`: the two paths must
			// not disagree on which way an omitted value falls.
			if (
				page.restricted_access === null ||
				page.restricted_access === undefined ||
				page.restricted_access === true
			) {
				fileListUnavailable = true
				folderState[prefix] = { loading: false, loaded: true }
				return
			}
			fileListUnavailable = false
			for (const folder of page.folders ?? []) {
				addEntry(folder.prefix, 'folder', folder.name)
			}
			for (const file of page.files ?? []) {
				if (regexFilter && !regexFilter.test(file.key)) continue
				addEntry(file.key, 'leaf', file.name)
			}
			folderState[prefix] = {
				loading: false,
				loaded: true,
				// An exhausted level serializes as JSON `null`, which is not `undefined`
				// — leaving it as-is makes "no more pages" look like another page and
				// sends the level round again with no token.
				nextPageToken: page.next_page_token ?? undefined
			}
		} catch (e) {
			if (stillCurrent()) {
				folderState[prefix] = { ...current, loading: false }
			}
			throw e
		} finally {
			// A response that lands after the user typed a filter must not rebuild the
			// list: flat mode owns `displayedFileKeys` then, and rebuilding it under
			// the lazy visibility rules would prune most of the search results.
			if (lazyMode && stillCurrent()) {
				refreshDisplayed()
			}
		}
	}

	/** Surface listing failures instead of leaving a folder looking empty. */
	function reportFolderError(prefix: string, e: unknown) {
		console.error('Error listing folder', prefix, e)
		sendUserToast(`Could not list ${prefix || 'the bucket root'}`, true)
	}

	function expandFolder(prefix: string) {
		loadFolderPage(prefix).catch((e) => reportFolderError(prefix, e))
	}

	function loadMore(prefix: string) {
		loadFolderPage(prefix, true).catch((e) => reportFolderError(prefix, e))
	}

	/**
	 * Bound on the pages fetched while hunting for one entry, so a preselected key
	 * that no longer exists cannot walk an entire bucket.
	 */
	const MAX_PAGES_WHILE_REVEALING = 20

	/**
	 * Page through `parent` until `child` shows up. A single page is not enough: the
	 * target can sort past the first page of its own folder, and giving up there
	 * leaves the selection looking absent.
	 */
	async function revealChild(parent: string, child: string, generation: number) {
		for (let fetched = 0; fetched < MAX_PAGES_WHILE_REVEALING; fetched++) {
			if (allFilesByKey[child] !== undefined) return
			const state = folderState[parent]
			if (state === undefined || !state.loaded) {
				await loadFolderPage(parent)
			} else if (state.nextPageToken !== undefined) {
				await loadFolderPage(parent, true)
			} else {
				return
			}
			if (generation !== listingGeneration) return
		}
	}

	/**
	 * Reveal a key by loading each folder above it in turn — with per-level listing
	 * an ancestor's children are not known until that level is fetched.
	 *
	 * `generation` is the listing this walk belongs to. A reveal is a chain of round
	 * trips, so the check has to be repeated after every one of them: a filter typed
	 * mid-walk switches the picker to the search, and the loads that follow would date
	 * themselves to that listing and graft browse results onto it.
	 */
	async function expandToKey(key: string, generation: number) {
		if (!key.startsWith(rootPath)) return
		const rest = key.slice(rootPath.length).split('/')
		let prefix = rootPath
		for (let i = 0; i < rest.length - 1; i++) {
			const child = prefix + rest[i] + '/'
			await revealChild(prefix, child, generation)
			if (generation !== listingGeneration) return
			prefix = child
			const info = allFilesByKey[prefix]
			// The folder is genuinely absent; deeper levels cannot exist either.
			if (info === undefined) break
			info.collapsed = false
		}
		if (allFilesByKey[key] === undefined) {
			await revealChild(prefix, key, generation)
			if (generation !== listingGeneration) return
		}
		refreshDisplayed()
	}

	let lastKeyFolders: string[] = $state([])

	/** Reports whether the listing completed, so a caller that moved the cursor can undo it. */
	async function loadFiles(): Promise<boolean> {
		if (lazyMode) {
			// Typing in the filter switches the picker to the flat listing while this is in
			// flight, and `loadFolderPage` resolves rather than throwing once superseded.
			// What follows belongs to the listing that started it: expanding a preselected
			// file would graft browse results into the search, and clearing the flags would
			// retire the search's spinner.
			const generation = listingGeneration
			fileListLoading = true
			try {
				await loadFolderPage(rootPath)
				if (
					generation === listingGeneration &&
					selectedFileKey !== undefined &&
					!emptyString(selectedFileKey.s3)
				) {
					await expandToKey(selectedFileKey.s3, generation)
				}
			} catch (e) {
				// `reloadContent` is called un-awaited from `open()`, so without this a
				// failing root listing surfaces as an unhandled rejection and an empty
				// tree indistinguishable from an empty bucket.
				reportFolderError(rootPath, e)
				return false
			} finally {
				if (generation === listingGeneration) {
					fileListLoading = false
					fileInfoLoading = false
				}
			}
			return true
		}
		// Same contract as the lazy branch above: every caller here is un-awaited, so an
		// uncaught rejection would leave the drawer on a spinner with nothing said. The
		// generation guard keeps a superseded listing from clearing the spinner of the one
		// that replaced it — `loadFlatFiles` returns early rather than throwing in that case.
		const generation = listingGeneration
		fileListLoading = true
		try {
			await loadFlatFiles()
		} catch (e) {
			console.error('Error listing files', e)
			sendUserToast('Could not list the files', true)
			// Nothing will load a preview now, so the right-hand pane has to stop waiting
			// too. On the other exits it is owned by whoever is still fetching metadata.
			if (generation === listingGeneration) {
				fileInfoLoading = false
			}
			return false
		} finally {
			if (generation === listingGeneration) {
				fileListLoading = false
			}
		}
		return true
	}

	/** Flat "Load more" / "Keep looking": a page that fails has to give the cursor back. */
	async function loadNextFlatPage() {
		// The page number alone does not identify our advance: a filter or storage change
		// resets the cursor, and the replacement listing can reach the same number before
		// this request fails. Rolling that one back would strand *its* cursor instead.
		const generation = listingGeneration
		page += 1
		const requested = page
		const ok = await loadFiles()
		// Only undo our own advance: another click may have moved it on meanwhile.
		if (!ok && generation === listingGeneration && page === requested) {
			page = requested - 1
		}
	}

	async function loadFlatFiles() {
		// Debounced searches overlap: an older response must not add its keys to a newer
		// search's results or overwrite its pagination state.
		const generation = listingGeneration
		fileListLoading = true
		let availableFiles = await listStoredFilesRequest({
			workspace: ws!,
			maxKeys: maxKeys, // fixed pages of 1000 files for now
			marker: page == 0 ? undefined : listMarkers[page - 1],
			// `prefix` is evaluated per path *segment* by the storage layer, so sending the
			// query there matched only whole folder names. `search` matches the raw key
			// prefix instead, which is what the box means.
			prefix: rootPath !== '' ? rootPath : undefined,
			search: filter.trim() !== '' ? filter.trim() : undefined,
			storage: storage,
			s3ResourcePath
		})
		if (generation !== listingGeneration) return
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
		for (let [index, file_path] of availableFiles.windmill_large_files.entries()) {
			if (regexFilter && !regexFilter.test(file_path.s3)) {
				continue
			}
			// Only count keys not already in the tree: a page can legitimately repeat
			// entries, and counting them again makes the total climb while the list
			// stays put.
			if (allFilesByKey[file_path.s3] === undefined) {
				displayedCount += 1
			}
			let split_path = file_path.s3.split('/')
			let parent_path: string | undefined = undefined
			let current_path: string | undefined = undefined
			let nestingLevel = 0

			if (index === availableFiles.windmill_large_files.length - 1 && split_path.length > 1) {
				lastKeyFolders = split_path.slice(0, -1)
			}

			for (let i = 0; i < split_path.length; i++) {
				parent_path = current_path
				current_path = current_path === undefined ? split_path[i] : current_path + split_path[i]

				if (i < split_path.length - 1) {
					current_path += '/'
				}

				nestingLevel = i * 2
				if (allFilesByKey[current_path] !== undefined) {
					allFilesByKey[current_path].count += 1
					continue
				}
				allFilesByKey[current_path] = {
					type: i === split_path.length - 1 ? 'leaf' : 'folder',
					full_key: current_path,
					display_name: split_path[i],
					collapsed: true, // folders collapsed by default
					parentPath: parent_path,
					nestingLevel: nestingLevel,
					count: 1
				}
				if (i == rootPathNestingLevel && current_path.startsWith(rootPath)) {
					displayedFileKeys.push(current_path)
				}
			}
		}
		// A short page means the listing is exhausted. Deriving "there is more" from
		// `count % maxKeys` instead would keep offering another page whenever the total
		// happens to be an exact multiple of the page size.
		// Searching returns an explicit cursor because it skips over keys that did not
		// match, so the last *returned* key is not where the next page resumes.
		const serverMarker = availableFiles.next_marker ?? undefined
		flatHasMore =
			serverMarker !== undefined ||
			(filter.trim() === '' && availableFiles.windmill_large_files.length === maxKeys)
		if (listMarkers.length == page) {
			count += availableFiles.windmill_large_files.length
			const nextMarker =
				serverMarker ??
				availableFiles.windmill_large_files?.[availableFiles.windmill_large_files.length - 1]?.s3
			if (nextMarker) listMarkers.push(nextMarker)
		}

		// before returning, un-collapse the folders containing the selected file (if any)
		if (selectedFileKey !== undefined && !emptyString(selectedFileKey.s3) && page === 0) {
			let split_path = selectedFileKey.s3.split('/')
			let current_path: string | undefined = undefined
			for (let i = 0; i < split_path.length; i++) {
				current_path = current_path === undefined ? split_path[i] : current_path + split_path[i]
				if (i < split_path.length - 1) {
					current_path += '/'
				}
				const folder = allFilesByKey[current_path]
				if (folder) {
					folder.collapsed = false
				}
				for (let file_key in allFilesByKey) {
					let file_info = allFilesByKey[file_key]
					if (file_info.parentPath === current_path) {
						displayedFileKeys.push(file_key)
					}
				}
			}
		}
		// The loop above only lists entries at the browsing root, so a later page's
		// keys land in `allFilesByKey` without ever being displayed — the folder they
		// belong to is already expanded and nothing re-scans it. Recomputing from the
		// expansion state picks them up without needing a collapse/expand round trip.
		// `displayedCount` is left alone: in this mode it counts files loaded, not rows shown.
		displayedFileKeys = computeVisibleKeys()
		fileListLoading = false
		fileInfoLoading = false
	}

	async function loadFileMetadataPlusPreviewAsync(fileKey: string | undefined) {
		if (fileKey === undefined || emptyString(fileKey)) {
			fileInfoLoading = false
			return
		}
		// The pane belongs to the newest request, not to a key: switching storage reloads
		// the same key, so comparing keys would let an older request speak for a newer one.
		const requestId = ++metadataRequestId
		fileInfoLoading = true
		let fileMetadataRaw: LoadFileMetadataResponse
		try {
			fileMetadataRaw = await loadFileMetadataRequest({
				workspace: ws!,
				fileKey: fileKey,
				storage: storage,
				s3ResourcePath
			})
		} catch (e) {
			// Every caller invokes this un-awaited, so a key that no longer exists would
			// otherwise leave the preview pane on "Loading..." forever.
			console.error('Error loading metadata for', fileKey, e)
			// Unless a later request has taken the pane over: it will report its own
			// outcome, including the loading flag, and blanking here would undo it.
			if (requestId !== metadataRequestId) {
				return
			}
			fileMetadata = undefined
			filePreview = undefined
			fileInfoLoading = false
			return
		}

		if (requestId !== metadataRequestId) {
			return
		}
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
		loadFilePreview(fileKey, requestId, fileMetadataRaw.size_in_bytes, fileMetadataRaw.mime_type)
	}

	async function loadFilePreview(
		fileKey: string,
		requestId: number,
		fileSizeInBytes?: number,
		fileMimeType?: string
	) {
		let filePreviewRaw = await loadFilePreviewRequest({
			workspace: ws!,
			fileKey: fileKey,
			fileSizeInBytes: fileSizeInBytes,
			fileMimeType: fileMimeType,
			csvSeparator: csvSeparatorChar,
			csvHasHeader: csvHasHeader,
			readBytesFrom: 0,
			readBytesLength: 128 * 1024, // For now static limit of 128Kb per file,
			storage: storage,
			s3ResourcePath
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

		if (requestId !== metadataRequestId) {
			return
		}
		if (filePreviewRaw !== undefined) {
			filePreview = {
				fileKey: fileKey,
				contentPreview: filePreviewContent,
				contentType: filePreviewRaw.content_type
			}
			if (fileMetadata) {
				fileMetadata.mimeType =
					((fileKey.endsWith('.png') ||
						fileKey.endsWith('.jpg') ||
						fileKey.endsWith('.jpeg') ||
						fileKey.endsWith('.webp')) &&
						'Image') ||
					(fileKey.endsWith('.pdf') && 'PDF') ||
					filePreview.contentType
			}
		}
		fileInfoLoading = false
	}

	async function deleteFileFromS3(fileKey: string | undefined) {
		fileDeletionInProgress = true
		if (fileKey === undefined) {
			return
		}
		try {
			await deleteS3FileRequest({
				workspace: ws!,
				fileKey: fileKey,
				storage: storage,
				s3ResourcePath
			})
		} finally {
			fileDeletionInProgress = false
			deletionModalOpen = false
		}
		sendUserToast(`${fileKey} deleted from S3 bucket`)
		selectedFileKey = { s3: '', storage }
		// The preview pane and its toolbar render from these rather than from the
		// selection, so the deleted file stays previewed — and offers to download, move
		// and delete itself — until they are cleared too.
		fileMetadata = undefined
		filePreview = undefined
		// A metadata load still in flight belongs to the file just deleted; retire it, or
		// its response repopulates the pane it was cleared from. Retiring it also means
		// nobody is left to report its outcome, so the pane's loading flag is ours.
		metadataRequestId += 1
		fileInfoLoading = false
		if (lazyMode) {
			// Only the level the file lived in changed; refetch it from its first page
			// and keep the rest of the expanded tree. Its already-fetched entries have
			// to go with the cursor, or the reset cursor would hand out a "Load more"
			// for pages that are still displayed.
			const parent = parentPathOf(fileKey) ?? rootPath
			for (const key of Object.keys(allFilesByKey)) {
				if (allFilesByKey[key].parentPath === (parent === '' ? undefined : parent)) {
					delete allFilesByKey[key]
				}
			}
			delete folderState[parent]
			// Invalidate this level so a pending "Load more" for it is not joined below.
			folderEpoch[parent] = (folderEpoch[parent] ?? 0) + 1
			delete inFlightFolderLoads[parent]
			await loadFolderPage(parent).catch((e) => reportFolderError(parent, e))
			return
		}
		const currentPage = page
		// Every page here has to land, starting with the fresh first one.
		if (await clearAndLoadFiles()) {
			for (let i = 0; i < currentPage; i++) {
				page = i + 1
				if (!(await loadFiles())) {
					// Stop at the last page that actually loaded.
					page = i
					break
				}
			}
		}
		const fileKeyFolders = fileKey.split('/').slice(0, -1)
		let current_path: string | undefined = undefined
		for (let i = 0; i < fileKeyFolders.length; i++) {
			current_path =
				current_path === undefined ? fileKeyFolders[i] : current_path + fileKeyFolders[i]
			if (i < fileKeyFolders.length) {
				current_path += '/'
			}
			const folder = allFilesByKey[current_path]
			if (folder) {
				folder.collapsed = false
			}
			for (let file_key in allFilesByKey) {
				let file_info = allFilesByKey[file_key]
				if (file_info.parentPath === current_path) {
					displayedFileKeys.push(file_key)
				}
			}
		}
		displayedFileKeys = [...new Set(displayedFileKeys)].sort()
	}

	/** Reports whether the fresh listing loaded, so a caller replaying pages can stop. */
	async function clearAndLoadFiles({
		keepFilter
	}: { keepFilter?: boolean } = {}): Promise<boolean> {
		// Anything already in flight belongs to the listing being discarded.
		listingGeneration += 1
		inFlightFolderLoads = {}
		folderEpoch = {}
		displayedFileKeys = []
		allFilesByKey = {}
		folderState = {}
		count = 0
		displayedCount = 0
		flatHasMore = false
		page = 0
		listMarkers = []
		fileMetadata = undefined
		filePreview = undefined
		if (!keepFilter) {
			filter = ''
		}
		return await loadFiles()
	}

	async function moveS3File(srcFileKey: string | undefined, destFileKey: string | undefined) {
		fileMoveInProgress = true
		if (srcFileKey === undefined || emptyString(destFileKey)) {
			return
		}
		try {
			await moveS3FileRequest({
				workspace: ws!,
				srcFileKey: srcFileKey,
				destFileKey: destFileKey!,
				storage: storage,
				s3ResourcePath
			})
		} finally {
			fileMoveInProgress = false
			moveModalOpen = false
		}
		sendUserToast(`${srcFileKey} moved to ${destFileKey}`)
		selectedFileKey = { s3: destFileKey!, storage }
		await clearAndLoadFiles()
		await loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
	}

	export async function open(_preSelectedFileKey: S3Object | undefined = undefined) {
		const preSelectedFileKey = _preSelectedFileKey && parseS3Object(_preSelectedFileKey)
		storage = preSelectedFileKey?.storage
		if (preSelectedFileKey !== undefined && preSelectedFileKey.s3.endsWith('/')) {
			rootPath = preSelectedFileKey.s3
			filter = ''
			selectedFileKey = undefined
		} else if (preSelectedFileKey !== undefined) {
			rootPath = ''
			initialFileKey = { ...preSelectedFileKey }
			selectedFileKey = { ...preSelectedFileKey }
		} else {
			rootPath = ''
		}
		reloadContent()
	}

	export async function close() {
		return selectedFileKey?.s3
			? {
					s3: selectedFileKey.s3,
					storage: storage
				}
			: undefined
	}

	export async function reloadContent() {
		if (initialFileKey !== undefined) {
			initialFileKeyInternalCopy = { ...initialFileKey }
		}
		fileListLoading = true
		try {
			await testConnectionRequest({
				workspace: ws!,
				storage: storage,
				s3ResourcePath
			})
			workspaceSettingsInitialized = true
		} catch (e) {
			fileListLoading = false
			console.error('Workspace not connected to object storage: ', e)
			workspaceSettingsInitialized = false
			return
		}
		await clearAndLoadFiles()
		if (selectedFileKey !== undefined) {
			const entry = allFilesByKey[selectedFileKey.s3]
			if (entry !== undefined) {
				if (entry.type !== 'folder') {
					loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
				}
			} else if (!lazyMode) {
				// Flat mode has listed everything it is going to, so a missing key really
				// is missing. Per-level listing has not: only the levels on the way to
				// the key were fetched, and `selectedFileKey` is bound out to the caller
				// — blanking it there would silently clear the configured object.
				selectedFileKey = { s3: '', storage }
			} else {
				loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
			}
		}
	}

	export async function selectAndClose() {
		if (selectedFileKey?.s3) {
			dispatch('selectAndClose', { s3: selectedFileKey.s3, storage })
		}
	}

	export async function exit() {
		if (initialFileKeyInternalCopy !== undefined) {
			selectedFileKey = { ...initialFileKeyInternalCopy }
		}
	}

	function selectItem(index: number, toggleCollapsed: boolean = true) {
		let item_key = displayedFileKeys[index]
		let item = allFilesByKey[item_key]
		if (item === undefined) return
		if (lazyMode) {
			if (item.type === 'folder') {
				if (folderOnly) {
					selectedFileKey = { s3: item_key, storage }
				}
				if (toggleCollapsed) {
					item.collapsed = !item.collapsed
				}
				if (!item.collapsed && !folderState[item_key]?.loaded) {
					// Children are unknown until this level is fetched.
					expandFolder(item_key)
				} else {
					refreshDisplayed()
				}
			} else {
				selectedFileKey = { s3: item_key, storage }
				loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
			}
			return
		}
		if (item.type === 'folder') {
			if (folderOnly) {
				selectedFileKey = {
					s3: item_key,
					storage
				}
			}
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
			displayedFileKeys = [...new Set(displayedFileKeys)].sort()
		} else {
			selectedFileKey = {
				s3: item_key,
				storage
			}
			loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
		}
	}
	$effect.pre(() => {
		filter != undefined && untrack(() => onFilterChange())
	})
</script>

{#if workspaceSettingsInitialized === false}
	{#if fromWorkspaceSettings}
		<Alert type="error" title="Connection to remote S3 bucket unsuccessful">
			<div class="flex flex-row gap-x-1 w-full items-center">
				<p class="text-clip grow min-w-0"> Double check the S3 resource fields and try again. </p>
			</div>
		</Alert>
	{:else if s3ResourcePath}
		<Alert type="error" title="Could not connect to the object storage of {s3ResourcePath}">
			<div class="flex flex-row gap-x-1 w-full items-center">
				<p class="text-clip grow min-w-0">
					Double check the resource fields and that its object storage is reachable, then try again.
				</p>
				<Button variant="default" on:click={reloadContent} startIcon={{ icon: RotateCw }} />
			</div>
		</Alert>
	{:else}
		<Alert type="error" title="Workspace not connected to any S3 storage">
			<div class="flex flex-row gap-x-1 w-full items-center">
				<p class="text-clip grow min-w-0">
					The workspace needs to be connected to an S3 storage to use this feature. You can <a
						target="_blank"
						href="{base}/workspace_settings?tab=windmill_lfs">configure it here</a
					>.
				</p>
				<Button variant="default" on:click={reloadContent} startIcon={{ icon: RotateCw }} />
			</div>
		</Alert>
	{/if}
{:else}
	{#if fileListUnavailable == true}
		{#if replaceUnauthorizedWarning}
			{@render replaceUnauthorizedWarning()}
		{:else}
			<div class="mb-2">
				<Alert type="info" title="Access to S3 bucket restricted">
					<p>
						You don't have access to the S3 bucket resource and your administrator has restricted
						the access to it. You are not authorized to browse the bucket content. If you think this
						is incorrect, please contact your workspace administrator.
					</p>
					<p>
						More info in <a
							href="https://www.windmill.dev/docs/core_concepts/persistent_storage/large_data_files"
							target="_blank">Windmill's documentation</a
						></p
					></Alert
				>
			</div>
		{/if}
	{/if}
	<div class="flex flex-row border rounded-md h-full min-h-0 overflow-hidden">
		{#if !fileListUnavailable}
			<div class="min-w-[30%] border-r flex flex-col min-h-0">
				{#if !rootPath}
					<div class="w-full p-1 border-b">
						<input
							type="text"
							placeholder="Search by path prefix"
							bind:value={filter}
							class="text-xl"
						/>
					</div>
				{/if}
				{#if displayedFileKeys.length === 0}
					{#if fileListLoading}
						<div class="grow min-h-0 flex justify-center items-center">
							<div class="flex text-secondary text-xs items-center">
								<Loader2 size={12} class="animate-spin mr-1" /> Loading content
							</div>
						</div>
					{:else}
						<div class="p-4 text-primary text-xs text-center italic">
							{#if filter.trim() !== ''}
								No files starting with "{filter.trim()}"
							{:else}
								No files in the workspace S3 bucket
							{/if}
						</div>
						{#if flatHasMore}
							<!-- A page can come back empty while keys remain — permission filtering
							can remove every match. Without this the only control that could resume
							the listing would be hidden behind the empty state. -->
							<div class="flex justify-center pb-4">
								<Button variant="default" size="xs2" on:click={() => loadNextFlatPage()}>
									Keep looking
								</Button>
							</div>
						{/if}
					{/if}
				{:else}
					<div class="grow min-h-0" bind:clientHeight={listDivHeight}>
						<VirtualList
							width="100%"
							height={listDivHeight}
							itemCount={displayedFileKeys.length}
							itemSize={42}
						>
							{#snippet header()}{/snippet}
							{#snippet footer()}{/snippet}
							{#snippet item({ index, style })}
								<!-- VirtualList can render an index past the array while the list is
								shrinking, so this must tolerate a missing key. -->
								{@const item_key = displayedFileKeys[index] ?? ''}
								{@const is_load_more = item_key.endsWith(LOAD_MORE_SUFFIX)}
								{@const load_more_prefix = is_load_more ? item_key.slice(0, -1) : ''}
								{@const file_info = allFilesByKey[item_key]}

								<div
									{style}
									class={twMerge(
										'hover:bg-surface-hover border-b',
										index === displayedFileKeys.length - 1 && 'border-b-0'
									)}
								>
									{#if is_load_more}
										<!-- Indented like the siblings it belongs to, so it must discount
										the browsing root the same way entry rows do. -->
										{@const loadMoreNesting =
											nestingLevelOf(load_more_prefix + 'x') - 2 * rootPathNestingLevel}
										{@const loadingMore = folderState[load_more_prefix]?.loading === true}
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											onclick={() => !loadingMore && loadMore(load_more_prefix)}
											class={twMerge(
												'flex flex-row h-full text-xs items-center justify-start text-secondary',
												loadingMore ? 'cursor-default' : 'cursor-pointer'
											)}
										>
											<div
												class="flex flex-row w-full gap-2 h-full items-center"
												style={`margin-left: ${(2 + loadMoreNesting) * 0.25}rem;`}
											>
												<!-- Occupies the same slot as the sibling rows' file/folder icon so
												the labels line up, and holds its width when the spinner swaps in. -->
												<div class="w-4 shrink-0 flex items-center justify-center">
													{#if loadingMore}
														<Loader2 size={16} class="animate-spin" />
													{:else}
														<ChevronDown size={16} />
													{/if}
												</div>
												<div class="truncate text-ellipsis w-56">Load more</div>
											</div>
										</div>
									{:else if file_info}
										{@const nestingLevel = file_info.nestingLevel - 2 * rootPathNestingLevel}
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											onclick={() => selectItem(index)}
											class={twMerge(
												'flex flex-row h-full font-semibold text-xs items-center justify-start',
												selectedFileKey !== undefined && selectedFileKey.s3 === file_info.full_key
													? 'bg-surface-hover'
													: ''
											)}
										>
											<div
												class={`flex flex-row w-full gap-2 h-full items-center`}
												style={`margin-left: ${(2 + nestingLevel) * 0.25}rem;`}
											>
												{#if file_info.type === 'folder'}
													{#if folderState[file_info.full_key]?.loading}
														<Loader2 size={16} class="animate-spin" />
													{:else if file_info.collapsed}<FolderClosed size={16} />{:else}<FolderOpen
															size={16}
														/>{/if}
													<div class="truncate text-ellipsis w-56">
														<!-- An object-store key may contain an empty segment, so `a//` is a real
														folder whose name is ''. Label it rather than rendering a blank row. -->
														{#if file_info.display_name === ''}
															<span class="italic text-secondary">(empty name)</span>
														{:else}{file_info.display_name}{/if}
														{#if !lazyMode}
															({file_info.count}{count % 1000 === 0 &&
															lastKeyFolders[file_info.nestingLevel / 2] === file_info.display_name
																? '+'
																: ''} item{file_info.count === 1 ? '' : 's'})
														{/if}
													</div>
												{:else}
													<FileIcon size={16} />
													<div class="truncate text-ellipsis w-56">
														{file_info.display_name}
													</div>
												{/if}
											</div>
										</div>
									{/if}
								</div>
							{/snippet}
						</VirtualList>
					</div>
					<div
						class="flex flex-col gap-2 text-2xs justify-center items-center text-secondary w-full border-t py-1"
					>
						{#if fileListLoading === true}
							<div class="flex text-secondary mt-1 text-xs justify-center items-center w-full">
								<Loader2 size={12} class="animate-spin mr-1" /> Loading content
							</div>
						{:else if lazyMode}
							<!-- Per-level listing: totals below the tree would be a count of what
							happens to be expanded, and each folder carries its own Load more row. -->
							<div>{displayedCount} item{displayedCount === 1 ? '' : 's'} shown</div>
						{:else}
							<div>
								{displayedCount}{flatHasMore ? '+' : ''}
								{displayedCount !== count ? 'filtered ' : ''}items (including inside folders)
							</div>

							{#if flatHasMore}
								<Button variant="default" size="xs2" on:click={() => loadNextFlatPage()}>
									Load more
								</Button>
							{/if}
						{/if}
					</div>
				{/if}
			</div>
		{/if}
		<div class="flex flex-col h-full w-full min-h-0 overflow-hidden">
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
				<div class="px-3 py-2 flex flex-col gap-2">
					<div class="flex flex-row items-center justify-between gap-2">
						<h2 class="text-emphasis text-sm font-semibold break-all min-w-0">
							{((p) => (p.startsWith(rootPath) ? p.slice(rootPath.length) : p))(
								fileMetadata.fileKey
							)}
						</h2>
						{#if filePreview !== undefined && (!hideS3SpecificDetails || !readOnlyMode || allowDelete)}
							<div class="flex gap-2 shrink-0">
								{#if !hideS3SpecificDetails}
									{@const downloadApiPath = `/w/${ws}/job_helpers/download_s3_file?file_key=${encodeURIComponent(fileMetadata?.fileKey ?? '')}${storage ? `&storage=${storage}` : ''}${s3ResourcePath ? `&s3_resource_path=${encodeURIComponent(s3ResourcePath)}` : ''}`}
									{@const downloadName =
										fileMetadata?.fileKey.split('/').pop() ?? 'unnamed_download.file'}
									{#if shouldDownloadViaClient()}
										<Button
											title="Download file from S3"
											variant="default"
											on:click={() => downloadViaClient(downloadApiPath, downloadName)}
											startIcon={{ icon: Download }}
											iconOnly={true}
										/>
									{:else}
										<Button
											title="Download file from S3"
											variant="default"
											href={`${base}/api${downloadApiPath}`}
											download={downloadName}
											startIcon={{ icon: Download }}
											iconOnly={true}
										/>
									{/if}
								{/if}
								{#if !readOnlyMode}
									<Button
										title="Move file"
										variant="default"
										on:click={() => {
											moveDestKey = fileMetadata?.fileKey ?? ''
											moveModalOpen = true
										}}
										startIcon={{ icon: MoveRight }}
										iconOnly={true}
									/>
								{/if}
								{#if !readOnlyMode || allowDelete}
									<Button
										title="Delete file"
										variant="default"
										on:click={() => {
											deletionModalOpen = true
										}}
										startIcon={{ icon: Trash }}
										iconOnly={true}
									/>
								{/if}
							</div>
						{/if}
					</div>
					{#if !hideS3SpecificDetails}
						<TableSimple
							headers={['Last modified', 'Size', 'Type']}
							data={[fileMetadata]}
							keys={['lastModified', 'sizeStr', 'mimeType']}
						/>
					{/if}
				</div>
			{/if}

			<!-- Visual preview extracted to a standalone S3FilePreview component
			     so the asset detail pane (and other surfaces) can render the
			     same image/PDF/CSV/text views without dragging in the rest
			     of the picker. The picker keeps its own metadata loading
			     above for the download/move toolbar; S3FilePreview does an
			     independent load — fine on this non-hot path, and avoids
			     plumbing pre-loaded state through component boundaries. -->
			<S3FilePreview
				fileKey={fileMetadata?.fileKey}
				{storage}
				{s3ResourcePath}
				workspace={ws}
				{loadFilePreviewRequest}
				{loadFileMetadataRequest}
				class="h-full"
			/>
		</div>
	</div>
{/if}

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
	loading={fileDeletionInProgress}
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
	loading={fileMoveInProgress}
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
			selectedFileKey = { s3: evt.detail, storage }
			await clearAndLoadFiles()
			loadFileMetadataPlusPreviewAsync(evt.detail)
		}
	}}
/>
