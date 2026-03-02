import type { StaticAppInput } from '../inputType'

export function collectStaticFields(fields: Record<string, StaticAppInput>) {
	return Object.fromEntries(
		Object.entries(fields ?? {})
			.filter(([k, v]) => v.type == 'static')
			.map(([k, v]) => {
				return [k, v['value']]
			})
	)
}

export type TriggerableV2 = {
	static_inputs: Record<string, any>
	one_of_inputs?: Record<string, any[] | undefined>
	allow_user_resources?: string[]
}

export async function hash(message) {
	try {
		const msgUint8 = new TextEncoder().encode(message) // encode as (utf-8) Uint8Array
		const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8) // hash the message
		const hashArray = Array.from(new Uint8Array(hashBuffer)) // convert buffer to byte array
		const hashHex = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
		return hashHex
	} catch {
		//subtle not available, trying pure js
		const { Sha256 } = await import('@aws-crypto/sha256-js')
		const hash = new Sha256()
		hash.update(message ?? '')
		const result = Array.from(await hash.digest())
		const hex = result.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
		return hex
	}
}
