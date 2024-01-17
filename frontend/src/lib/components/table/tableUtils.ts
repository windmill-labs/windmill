export function isLink(value: string) {
	return value?.startsWith('http://') || value?.startsWith('https://')
}

export function isEmail(value: string) {
	return value?.includes('@')
}
