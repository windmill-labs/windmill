export function blobReader(blob, onChunk, chunkSize = 1024 * 1024) {
    return new Promise((resolve, reject) => {
        const fileReader = new FileReader();
        fileReader.addEventListener("error", reject);
        fileReader.addEventListener("abort", reject);
        const size = blob.size;
        let totalBytesRead = 0;
        function read() {
            if (totalBytesRead >= size) {
                resolve();
                return;
            }
            fileReader.readAsArrayBuffer(blob.slice(totalBytesRead, Math.min(size, totalBytesRead + chunkSize)));
        }
        fileReader.addEventListener("load", (event) => {
            const result = event.target.result;
            onChunk(new Uint8Array(result));
            totalBytesRead += result.byteLength;
            read();
        });
        read();
    });
}
