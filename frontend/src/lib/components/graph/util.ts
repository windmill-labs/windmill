export const NODE = {
	width: 275,
	height: 34,
	gap: {
		horizontal: 40,
		vertical: 50
	}
}

export function* createIdGenerator(): Generator<number, number, unknown> {
	let id = 0
	while (true) {
		yield id++
	}
}

