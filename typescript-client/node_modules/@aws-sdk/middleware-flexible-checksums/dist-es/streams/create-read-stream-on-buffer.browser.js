export function createReadStreamOnBuffer(buffer) {
    return new Blob([buffer]).stream();
}
