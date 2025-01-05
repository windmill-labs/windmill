import { inflate as pakoInflate } from 'pako'

declare class DecompressionStream {
	constructor(format: 'gzip' | 'deflate' | 'deflate-raw')
	writable: WritableStream
	readable: ReadableStream
}

function toUint8Array(data: ArrayBuffer): Uint8Array {
	return new Uint8Array(data)
}

async function inflate(data: Uint8Array | ArrayBuffer): Promise<Uint8Array> {
	if (typeof DecompressionStream !== 'undefined') {
		const ds = new DecompressionStream('gzip')
		const writer = ds.writable.getWriter()
		writer.write(data)
		writer.close()
		return new Response(ds.readable).arrayBuffer().then(toUint8Array)
	} else {
		return pakoInflate(data)
	}
}

interface Entry {
	path: string
	content: string
}

const decoder = /* @__PURE__ */ new TextDecoder()

function read_string(view: DataView, offset: number, length: number): string {
	const temp = new Uint8Array(view.buffer, offset, length)
	const i = temp.indexOf(0)
	return decoder.decode(temp.subarray(0, i === -1 ? length : i))
}

function read_pax(view: DataView, offset: number, length: number) {
	const re = /^(\d+) ([^=]+)=(.*)$/gm
	const raw = read_string(view, offset, length)
	const result: { path?: string; size?: number } = { path: undefined, size: undefined }
	for (let match: RegExpExecArray | null; (match = re.exec(raw)); ) {
		const [, _len, key, value] = match
		if (key === 'path') result.path = value
		if (key === 'size') result.size = Number.parseInt(value)
	}
	return result
}

const normalize = (path: string) => path.replace(/\\+/g, '/').replace(/^\.\/|^\./, '')

export async function read_tarball(item: Uint8Array): Promise<Entry[]> {
	const entries: Entry[] = []

	if (item[0] === 0x1f && item[1] === 0x8b) {
		item = await inflate(item)
	}

	const length = item.byteLength
	const view = new DataView(item.buffer)

	let globalPaxHeader: { path?: string; size?: number } | undefined
	let localPaxHeader: { path?: string; size?: number } | undefined
	for (let offset = 0; offset + 4 < length && view.getUint32(offset, true) !== 0; ) {
		let name: string
		let content = ''
		let size: number

		name = read_string(view, offset, 100)
		// const mode = Number.parseInt(read_string(view, offset + 100, 8), 8)
		size = Number.parseInt(read_string(view, offset + 124, 12), 8)
		const type = read_string(view, offset + 156, 1)

		const ustar = read_string(view, offset + 257, 6)
		if (ustar.includes('ustar')) {
			const prefix = read_string(view, offset + 345, 155)
			if (prefix.length > 0) name = prefix + '/' + name
		}

		offset += 512
		if (type === '0' || type === '') {
			content = read_string(view, offset, size)
		} else if (type === 'g') {
			globalPaxHeader = read_pax(view, offset, size)
		} else if (type === 'x') {
			localPaxHeader = read_pax(view, offset, size)
		}
		offset += Math.ceil(size / 512) * 512

		if (type === 'g' || type === 'x') {
			continue
		}

		if (globalPaxHeader) {
			if (globalPaxHeader.path) name = globalPaxHeader.path
			if (globalPaxHeader.size) size = globalPaxHeader.size
		}

		if (localPaxHeader) {
			if (localPaxHeader.path) name = localPaxHeader.path
			if (localPaxHeader.size) size = localPaxHeader.size
			localPaxHeader = undefined
		}

		if (type === '0' || type === '') {
			name = normalize(name)
			// Line below strips the first segment of the path, this exists only to support npm package contents.
			// Arbitrary tarballs may not have this structure.
			name = name.replace(/^[^/]+\/?/, '')
			// Line below dirty fixes a bug in some packages where the path starts with './'
			name = name.replace(/^\.\//, '')
			entries.push({ path: name, content })
		}
	}

	return entries
}
