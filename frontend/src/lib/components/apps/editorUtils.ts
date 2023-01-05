export function defaultCode(component: string, language: string): string | undefined {
    if (component === 'tablecomponent' && language === 'deno') {
        return `export async function main(x: string) {
  return [
    { foo: x, bar: 42 },
    { foo: "static", bar: 84 }]
}
`
    }
    return undefined
}