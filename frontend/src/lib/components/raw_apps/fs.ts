// Credits: https://github.com/esbuild/esbuild.github.io/blob/main/src/try/fs.ts
// This file contains a hack to get the "esbuild-wasm" package to run in the
// browser with file system support. Although there is no API for this, it
// can be made to work anyway by pretending that node's "fs" API is present.

let nextFD = 3
let nextInode = 1

const enum Kind {
	File,
	Directory
}

const enum StatsMode {
	IFREG = 0o100000,
	IFDIR = 0o40000
}

type Entry = File | Directory

interface Metadata {
	inode_: number
	ctime_: Date
	mtime_: Date
}

interface File extends Metadata {
	kind_: Kind.File
	content_: Uint8Array
}

interface Directory extends Metadata {
	kind_: Kind.Directory
	children_: Map<string, Entry>
}

class Stats {
	declare dev: number
	declare ino: number
	declare mode: number
	declare nlink: number
	declare uid: number
	declare gid: number
	declare rdev: number
	declare size: number
	declare blksize: number
	declare blocks: number
	declare atimeMs: number
	declare mtimeMs: number
	declare ctimeMs: number
	declare birthtimeMs: number
	declare atime: Date
	declare mtime: Date
	declare ctime: Date
	declare birthtime: Date

	constructor(entry: Entry) {
		const blksize = 4096
		const size = entry.kind_ === Kind.File ? entry.content_.length : 0
		const mtimeMs = entry.mtime_.getTime()
		const ctimeMs = entry.ctime_.getTime()
		this.dev = 1
		this.ino = entry.inode_
		this.mode = entry.kind_ === Kind.File ? StatsMode.IFREG : StatsMode.IFDIR
		this.nlink = 1
		this.uid = 1
		this.gid = 1
		this.rdev = 0
		this.size = size
		this.blksize = blksize
		this.blocks = (size + (blksize - 1)) & (blksize - 1)
		this.atimeMs = mtimeMs
		this.mtimeMs = mtimeMs
		this.ctimeMs = ctimeMs
		this.birthtimeMs = ctimeMs
		this.atime = entry.mtime_
		this.mtime = entry.mtime_
		this.ctime = entry.ctime_
		this.birthtime = entry.ctime_
	}

	isDirectory(): boolean {
		return this.mode === StatsMode.IFDIR
	}

	isFile(): boolean {
		return this.mode === StatsMode.IFREG
	}
}

interface Handle {
	entry_: Entry
	offset_: number
}

const EBADF = errorWithCode('EBADF')
const EINVAL = errorWithCode('EINVAL')
const EISDIR = errorWithCode('EISDIR')
const ENOENT = errorWithCode('ENOENT')
const ENOTDIR = errorWithCode('ENOTDIR')
const handles = new Map<number, Handle>()
const encoder = new TextEncoder()
const decoder = new TextDecoder()
const root: Directory = createDirectory()

export let stderrSinceReset = ''

// The "esbuild-wasm" package overwrites "fs.writeSync" with this value
let esbuildWriteSync: (
	fd: number,
	buffer: Uint8Array,
	offset: number,
	length: number,
	position: number | null
) => void

// The "esbuild-wasm" package overwrites "fs.read" with this value
let esbuildRead: (
	fd: number,
	buffer: Uint8Array,
	offset: number,
	length: number,
	position: number | null,
	callback: (err: Error | null, count: number, buffer: Uint8Array) => void
) => void

function writeSync(
	fd: number,
	buffer: Uint8Array,
	offset: number,
	length: number,
	position: number | null
): void {
	if (fd <= 2) {
		if (fd === 2) writeToStderr(buffer, offset, length)
		else esbuildWriteSync(fd, buffer, offset, length, position)
	} else {
		throw EINVAL
	}
}

function read(
	fd: number,
	buffer: Uint8Array,
	offset: number,
	length: number,
	position: number | null,
	callback: (err: Error | null, count: number, buffer: Uint8Array) => void
): void {
	if (fd <= 2) {
		esbuildRead(fd, buffer, offset, length, position, callback)
	} else {
		const handle = handles.get(fd)
		if (!handle) {
			callback(EBADF, 0, buffer)
		} else if (handle.entry_.kind_ === Kind.Directory) {
			callback(EISDIR, 0, buffer)
		} else {
			const content = handle.entry_.content_
			if (position !== null && position !== -1) {
				const slice = content.slice(position, position + length)
				buffer.set(slice, offset)
				callback(null, slice.length, buffer)
			} else {
				const slice = content.slice(handle.offset_, handle.offset_ + length)
				handle.offset_ += slice.length
				buffer.set(slice, offset)
				callback(null, slice.length, buffer)
			}
		}
	}
}

function rejectConflict(part: string): never {
	throw new Error(JSON.stringify(part) + ' cannot be both a file and a directory')
}

export function resetFileSystem(files: Record<string, string>): void {
	root.children_.clear()
	stderrSinceReset = ''

	for (const path in files) {
		const parts = splitPath(absoluteNormalizedPath(path))
		let dir = root

		for (let i = 0; i + 1 < parts.length; i++) {
			const part = parts[i]
			let child = dir.children_.get(part)
			if (!child) {
				child = createDirectory()
				dir.children_.set(part, child)
			} else if (child.kind_ !== Kind.Directory) {
				rejectConflict(part)
			}
			dir = child
		}

		const part = parts[parts.length - 1]
		if (dir.children_.has(part)) rejectConflict(part)
		dir.children_.set(part, createFile(encoder.encode(files[path])))
	}
}

declare global {
	// eslint-disable-next-line no-var
	var fs: unknown
}

// Override the same variable in the "esbuild-wasm" package
globalThis.fs = {
	get writeSync() {
		return writeSync
	},
	set writeSync(value) {
		esbuildWriteSync = value
	},

	get read() {
		return read
	},
	set read(value) {
		esbuildRead = value
	},

	constants: {
		O_WRONLY: -1,
		O_RDWR: -1,
		O_CREAT: -1,
		O_TRUNC: -1,
		O_APPEND: -1,
		O_EXCL: -1
	},

	open(
		path: string,
		flags: string | number,
		mode: string | number,
		callback: (err: Error | null, fd: number | null) => void
	) {
		try {
			const entry = getEntryFromPath(path)
			const fd = nextFD++
			handles.set(fd, { entry_: entry, offset_: 0 })
			callback(null, fd)
		} catch (err) {
			callback(err, null)
		}
	},

	close(fd: number, callback: (err: Error | null) => void) {
		callback(handles.delete(fd) ? null : EBADF)
	},

	write(
		fd: number,
		buffer: Uint8Array,
		offset: number,
		length: number,
		position: number | null,
		callback: (err: Error | null, count: number, buffer: Uint8Array) => void
	) {
		if (fd <= 2) {
			if (fd === 2) writeToStderr(buffer, offset, length)
			else esbuildWriteSync(fd, buffer, offset, length, position)
			callback(null, length, buffer)
		} else {
			callback(EINVAL, 0, buffer)
		}
	},

	readdir(path: string, callback: (err: Error | null, files: string[] | null) => void) {
		try {
			const entry = getEntryFromPath(path)
			if (entry.kind_ !== Kind.Directory) throw ENOTDIR
			callback(null, [...entry.children_.keys()])
		} catch (err) {
			callback(err, null)
		}
	},

	stat(path: string, callback: (err: Error | null, stats: Stats | null) => void) {
		try {
			const entry = getEntryFromPath(path)
			callback(null, new Stats(entry))
		} catch (err) {
			callback(err, null)
		}
	},

	lstat(path: string, callback: (err: Error | null, stats: Stats | null) => void) {
		try {
			const entry = getEntryFromPath(path)
			callback(null, new Stats(entry))
		} catch (err) {
			callback(err, null)
		}
	},

	fstat(fd: number, callback: (err: Error | null, stats: Stats | null) => void) {
		const handle = handles.get(fd)
		if (handle) {
			callback(null, new Stats(handle.entry_))
		} else {
			callback(EBADF, null)
		}
	}
}

function createFile(content: Uint8Array): File {
	const now = new Date()
	return {
		kind_: Kind.File,
		inode_: nextInode++,
		ctime_: now,
		mtime_: now,
		content_: content
	}
}

function createDirectory(): Directory {
	const now = new Date()
	return {
		kind_: Kind.Directory,
		inode_: nextInode++,
		ctime_: now,
		mtime_: now,
		children_: new Map()
	}
}

function absoluteNormalizedPath(path: string): string {
	if (path[0] !== '/') path = '/' + path
	const parts = path.split('/')
	parts.shift()
	let end = 0
	for (let i = 0; i < parts.length; i++) {
		const part = parts[i]
		if (part === '..') {
			if (end) end--
		} else if (part !== '.' && part !== '') {
			parts[end++] = part
		}
	}
	parts.length = end
	return '/' + parts.join('/')
}

function splitPath(path: string): string[] {
	path = absoluteNormalizedPath(path)
	if (path === '/') return []
	const parts = path.split('/')
	parts.shift()
	return parts
}

function getEntryFromPath(path: string): Entry {
	const parts = splitPath(path)
	let dir = root
	for (let i = 0, n = parts.length; i < n; i++) {
		const child = dir.children_.get(parts[i])
		if (!child) throw ENOENT
		if (child.kind_ === Kind.File) {
			if (i + 1 === n) return child
			throw ENOTDIR
		}
		dir = child
	}
	return dir
}

function errorWithCode(code: string): Error {
	const err = new Error(code) as any
	err.code = code
	return err
}

function writeToStderr(buffer: Uint8Array, offset: number, length: number): void {
	stderrSinceReset += decoder.decode(
		offset === 0 && length === buffer.length ? buffer : buffer.slice(offset, offset + length)
	)
}
