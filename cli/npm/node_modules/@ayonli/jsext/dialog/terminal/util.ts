export function escape(str: string) {
    return str.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}
