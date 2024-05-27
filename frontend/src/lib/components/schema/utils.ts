export function testRegex(pattern: string, value: any): boolean {
	try {
		const regex = new RegExp(pattern)
		return regex.test(value)
	} catch (err) {
		return false
	}
}

export function evalValueToRaw(
	inputCategory: string,
	value: any,
	isListJson: boolean
): string | undefined {
	return inputCategory === 'object' ||
		inputCategory === 'resource-object' ||
		(inputCategory == 'list' && !isListJson)
		? JSON.stringify(value, null, 2)
		: undefined
}
