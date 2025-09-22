const aCharCode = 'a'.charCodeAt(0)

export const forbiddenIds: string[] = [
	'do',
	'bg',
	'ctx',
	'state',
	'if',
	'else',
	'for',
	'delete',
	'while',
	'new',
	'in',
	'failure',
	'preprocessor',
	'as'
]

export function numberToChars(n: number) {
	if (n < 0) {
		return '-' + numberToChars(-n)
	}
	var b = [n],
		sp,
		out,
		i,
		div

	sp = 0
	while (sp < b.length) {
		if (b[sp] > 25) {
			div = Math.floor(b[sp] / 26)
			b[sp + 1] = div - 1
			b[sp] %= 26
		}
		sp += 1
	}

	out = ''
	for (i = 0; i < b.length; i += 1) {
		let charCode = aCharCode + b[i]
		out = String.fromCharCode(charCode) + out
	}

	return out
}

export function getNextId(currentKeys: string[]): string {
	const takenKeys = [...currentKeys, ...forbiddenIds].map(charsToNumber)
	let i = charsToNumber('a')
	while (true) {
		if (!takenKeys.includes(i)) {
			break
		}
		i++
	}
	return numberToChars(i)
}

export function charsToNumber(n: string): number {
	if (n.charAt(0) == '-') {
		return charsToNumber(n.slice(1)) * -1
	}
	let b = Math.pow(26, n.length - 1)
	let res = 0
	for (let c of n) {
		let charCode = c == '-' || c == '_' ? aCharCode + 25 : c.charCodeAt(0)
		res += (charCode - aCharCode + 1) * b
		b = b / 26
	}
	return res - 1
}
