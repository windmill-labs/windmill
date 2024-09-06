import type {
    DialogOptions,
    DownloadFileOptions,
    FileDialogOptions,
    PickFileOptions,
    ProgressAbortHandler,
    ProgressFunc,
    ProgressState,
    PromptOptions,
    SaveFileOptions,
} from "../dialog.ts";

export type {
    DialogOptions,
    DownloadFileOptions,
    FileDialogOptions,
    PickFileOptions,
    ProgressAbortHandler,
    ProgressFunc,
    ProgressState,
    PromptOptions,
    SaveFileOptions,
};

export async function alert(message: string, options: DialogOptions = {}): Promise<void> {
    void message, options;
    throw new Error("Unsupported runtime");
}

export async function confirm(message: string, options: DialogOptions = {}): Promise<boolean> {
    void message, options;
    throw new Error("Unsupported runtime");
}

export async function prompt(
    message: string,
    defaultValue?: string | undefined
): Promise<string | null>;
export async function prompt(message: string, options?: PromptOptions): Promise<string | null>;
export async function prompt(
    message: string,
    options: string | PromptOptions = ""
): Promise<string | null> {
    void message, options;
    throw new Error("Unsupported runtime");
}

export async function progress<T>(
    message: string,
    fn: ProgressFunc<T>,
    onAbort: ProgressAbortHandler<T> | undefined = undefined
): Promise<T | null> {
    void message, fn, onAbort;
    throw new Error("Unsupported runtime");
}

export async function pickFile(
    options: PickFileOptions = {}
): Promise<string | FileSystemFileHandle | null> {
    void options;
    throw new Error("Unsupported runtime");
}

export async function pickFiles(
    options: FileDialogOptions = {}
): Promise<string[] | FileSystemFileHandle[]> {
    void options;
    throw new Error("Unsupported runtime");
}

export async function pickDirectory(
    options: Pick<FileDialogOptions, "title"> = {}
): Promise<string | FileSystemDirectoryHandle | null> {
    void options;
    throw new Error("Unsupported runtime");
}

export function openFile(options?: FileDialogOptions): Promise<File | null>;
/**
 * @deprecated use {@link openFiles} instead.
 */
export function openFile(options: FileDialogOptions & {
    multiple: true;
}): Promise<File[]>;
/**
 * @deprecated use {@link openDirectory} instead.
 */
export function openFile(options: Pick<FileDialogOptions, "title"> & {
    directory: true;
}): Promise<File[]>;
export async function openFile(options: FileDialogOptions & {
    multiple?: boolean;
    directory?: boolean;
} = {}): Promise<File | File[] | null> {
    void options;
    throw new Error("Unsupported runtime");
}

export async function openFiles(options: FileDialogOptions = {}): Promise<File[]> {
    void options;
    throw new Error("Unsupported runtime");
}

export async function openDirectory(
    options: Pick<FileDialogOptions, "title"> = {}
): Promise<File[]> {
    void options;
    throw new Error("Unsupported runtime");
}

export async function saveFile(file: File, options?: Pick<SaveFileOptions, "title">): Promise<void>;
export async function saveFile(
    file: Blob | ArrayBuffer | ArrayBufferView | ReadableStream<Uint8Array>,
    options?: SaveFileOptions
): Promise<void>;
export async function saveFile(
    file: File | Blob | ArrayBuffer | ArrayBufferView | ReadableStream<Uint8Array>,
    options: SaveFileOptions = {}
): Promise<void> {
    void file, options;
    throw new Error("Unsupported runtime");
}

export async function downloadFile(
    url: string | URL,
    options: DownloadFileOptions = {}
): Promise<void> {
    void url, options;
    throw new Error("Unsupported runtime");
}
