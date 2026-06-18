export function wrapDucklakeQuery(query: string, ducklake: string): string {
	let attach = `ATTACH 'ducklake://${ducklake}' AS dl;USE dl;\n`
	return query.replace(/^(--.*\n)*/, (match) => match + attach)
}
