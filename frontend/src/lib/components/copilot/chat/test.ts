//asdasd
import * as wmill from 'windmill-client'

export async function main(
	a: number,
	b: 'my' | 'enum',
	e = 'inferred type string from default arg',
	f = { nested: 'object' },
	g:
		| {
				label: 'Variant 1'
				foo: string
		  }
		| {
				label: 'Variant 2'
				bar: number
		  }
) {
	return { foo: a }
}
