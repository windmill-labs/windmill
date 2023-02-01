import type { Script } from "./gen"

export type OwnerKind = 'group' | 'user' | 'folder'

export type ActionKind = 'Create' | 'Update' | 'Delete' | 'Execute'

export type SupportedLanguage = Script.language

export interface SchemaProperty {
	type: string | undefined
	description: string
	pattern?: string
	default?: any
	enum?: string[]
	contentEncoding?: 'base64' | 'binary'
	format?: string
	items?: { type?: 'string' | 'number' | 'bytes'; contentEncoding?: 'base64' }
	properties?: { [name: string]: SchemaProperty }
}

export type Schema = {
	$schema: string | undefined
	type: string
	properties: { [name: string]: SchemaProperty }
	required: string[]
}

export type Meta = { ownerKind: OwnerKind; owner: string; name: string }

export function pathToMeta(path: string): Meta {
	const splitted = path.split('/')
	let ownerKind: OwnerKind
	if (splitted[0] == 'g') {
		ownerKind = 'group'
	} else if (splitted[0] == 'f') {
		ownerKind = 'folder'
	} else if (splitted[0] == 'u') {
		ownerKind = 'user'
	} else {
		console.error('Not recognized owner:' + splitted[0])
		return {
			ownerKind: 'user',
			owner: '',
			name: ''
		}
	}
	return {
		ownerKind,
		owner: splitted[1],
		name: splitted.slice(2).join('/')
	}
}
