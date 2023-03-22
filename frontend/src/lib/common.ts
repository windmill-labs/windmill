import type { Script } from "./gen"

export type OwnerKind = 'group' | 'user' | 'folder'

export type ActionKind = 'Create' | 'Update' | 'Delete' | 'Execute'

export type SupportedLanguage = Script.language

export interface PropertyDisplayInfo {
	property: SchemaProperty
	name: string
	isRequired: boolean
	depth: number
	index: number
	propertiesNumber: number
}

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
	required?: string[]
}

export type Schema = {
	$schema: string | undefined
	type: string
	properties: { [name: string]: SchemaProperty }
	required: string[]
}

export type Meta = { ownerKind: OwnerKind; owner: string; name: string }

type Enumerate<N extends number, Acc extends number[] = []> = Acc['length'] extends N
	? Acc[number]
	: Enumerate<N, [...Acc, Acc['length']]>

/** An inclusive range of integer numbers */
export type IntRange<F extends number, T extends number> = F | Exclude<Enumerate<T>, Enumerate<F>> | T

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
