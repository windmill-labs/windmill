import {
	type IFileSystemProviderWithFileReadWriteCapability,
	FileType,
	type IFileChange,
	type IFileDeleteOptions,
	type IFileOverwriteOptions,
	type IFileWriteOptions,
	type IStat,
	type IWatchOptions,
	FileSystemProviderError,
	FileSystemProviderErrorCode
} from '@codingame/monaco-vscode-files-service-override'

import { EventEmitter, Uri } from 'vscode'
import type { Event } from 'vscode/vscode/vs/base/common/event'
import type { IDisposable } from 'vscode/vscode/vs/base/common/lifecycle'
import type { URI } from 'vscode/vscode/vs/base/common/uri'
import type { FileSystemProviderCapabilities } from 'vscode/vscode/vs/platform/files/common/files'

interface FileNode {
	type: FileType
	name: string
	content?: string
	children?: Map<string, FileNode>
}

export class ReadOnlyMemoryFileSystemProvider
	implements IFileSystemProviderWithFileReadWriteCapability
{
	private readonly _onDidChangeFile = new EventEmitter<readonly IFileChange[]>()
	private readonly _onDidChangeCapabilities = new EventEmitter<void>()
	private readonly _root: FileNode

	constructor(private fileSystem: Record<string, string>) {
		this._root = {
			type: FileType.Directory,
			name: '',
			children: new Map()
		}
		this._buildTree()
	}

	public rebuildTree(fs?: Record<string, string>): void {
		this._root.children?.clear()
		if (fs) {
			this.fileSystem = fs
		}
		this._buildTree()
		// Notify watchers that everything might have changed
		this._onDidChangeFile.fire([{ type: 0, resource: Uri.file('/') }])
	}

	private _buildTree() {
		const fileSystem = this.fileSystem
		for (const [path, content] of Object.entries(fileSystem)) {
			// Normalize path to always start with / and split into segments
			const normalizedPath = path.startsWith('/') ? path.slice(1) : path
			const segments = normalizedPath.split('/')

			let currentNode = this._root

			// Create/traverse path segments
			for (let i = 0; i < segments.length; i++) {
				const segment = segments[i]
				const isLast = i === segments.length - 1

				if (!currentNode.children) {
					currentNode.children = new Map()
				}

				if (isLast) {
					// Add file
					currentNode.children.set(segment, {
						type: FileType.File,
						name: segment,
						content
					})
				} else {
					// Create/traverse directory
					if (!currentNode.children.has(segment)) {
						currentNode.children.set(segment, {
							type: FileType.Directory,
							name: segment,
							children: new Map()
						})
					}
					currentNode = currentNode.children.get(segment)!
				}
			}
		}
	}

	private _findNode(path: string): FileNode | undefined {
		console.log('FIND NODE', path)
		if (path === '/' || path === '') {
			return this._root
		}

		const segments = path.startsWith('/') ? path.slice(1).split('/') : path.split('/')
		let currentNode = this._root
		for (const segment of segments) {
			if (!currentNode.children?.has(segment)) {
				return undefined
			}
			currentNode = currentNode.children.get(segment)!
		}

		return currentNode
	}

	// Required capabilities
	capabilities: FileSystemProviderCapabilities = 2048 | 1024
	onDidChangeCapabilities: Event<void> = this._onDidChangeCapabilities.event
	onDidChangeFile: Event<readonly IFileChange[]> = this._onDidChangeFile.event
	onDidWatchError?: Event<string>

	async readFile(resource: URI): Promise<Uint8Array> {
		const path = this.getPath(resource)
		console.log('READ FILE', resource, path)

		const node = this._findNode(path)
		if (!node || node.type !== FileType.File || !node.content) {
			throw FileSystemProviderError.create(
				'File not found',
				FileSystemProviderErrorCode.FileNotFound
			)
		}

		return new TextEncoder().encode(node.content)
	}

	async writeFile(resource: URI, content: Uint8Array, opts: IFileWriteOptions): Promise<void> {
		throw new Error('Filesystem is read-only')
	}

	async stat(resource: URI): Promise<IStat> {
		const path = this.getPath(resource)
		console.log('STAT', resource, path)

		const node = this._findNode(path)
		if (!node) {
			throw FileSystemProviderError.create(
				'File not found',
				FileSystemProviderErrorCode.FileNotFound
			)
			// throw new Error(`Path not found: ${path}`)
		}

		return {
			type: node.type,
			ctime: Date.now(),
			mtime: Date.now(),
			size: node.content ? new TextEncoder().encode(node.content).length : 0
		}
	}

	async readdir(resource: URI): Promise<[string, FileType][]> {
		const path = this.getPath(resource)
		console.log('READDIR', path)

		const node = this._findNode(path)
		if (!node || node.type !== FileType.Directory || !node.children) {
			throw FileSystemProviderError.create(
				'Directory not found',
				FileSystemProviderErrorCode.FileNotFound
			)
		}

		return Array.from(node.children.entries()).map(([name, child]) => [name, child.type])
	}

	watch(resource: URI, opts: IWatchOptions): IDisposable {
		return { dispose: () => {} }
	}

	async mkdir(resource: URI): Promise<void> {
		throw new Error('Filesystem is read-only')
	}

	async delete(resource: URI, opts: IFileDeleteOptions): Promise<void> {
		throw new Error('Filesystem is read-only')
	}

	async rename(from: URI, to: URI, opts: IFileOverwriteOptions): Promise<void> {
		throw new Error('Filesystem is read-only')
	}

	private getPath(uri: URI): string {
		return uri.path
	}
}
