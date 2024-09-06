import CancelButton from "./CancelButton.ts";
import Dialog, { closeDialog } from "./Dialog.ts";
import Footer from "./Footer.ts";
import Input from "./Input.ts";
import OkButton from "./OkButton.ts";
import Progress from "./Progress.ts";
import Text from "./Text.ts";
import type { ProgressFunc, ProgressState } from "../../dialog.ts";

export async function alertInBrowser(message: string) {
    await new Promise<void>(resolve => {
        document.body.appendChild(
            Dialog(
                {
                    onCancel: () => resolve(),
                    onOk: () => resolve(),
                },
                Text(message),
                Footer(
                    OkButton()
                )
            )
        );
    });
}

export async function confirmInBrowser(message: string) {
    return new Promise<boolean>(resolve => {
        document.body.appendChild(
            Dialog(
                {
                    onCancel: () => resolve(false),
                    onOk: () => resolve(true),
                },
                Text(message),
                Footer(
                    CancelButton(),
                    OkButton()
                )
            )
        );
    });
}

export async function promptInBrowser(message: string, options: {
    type: "text" | "password";
    defaultValue?: string | undefined;
}) {
    const { type, defaultValue } = options;
    return new Promise<string | null>(resolve => {
        document.body.appendChild(
            Dialog(
                {
                    onCancel: () => resolve(null),
                    onOk: (dialog: HTMLDialogElement) => {
                        const input = dialog.querySelector("input") as HTMLInputElement;
                        resolve(input.value);
                    },
                },
                Text(message),
                Input({ type, value: defaultValue }),
                Footer(
                    CancelButton(),
                    OkButton()
                )
            )
        );
    });
}

export async function progressInBrowser<T>(message: string, fn: ProgressFunc<T>, options: {
    signal: AbortSignal;
    abort?: (() => void) | undefined;
    listenForAbort?: (() => Promise<T>) | undefined;
}) {
    const { signal, abort, listenForAbort } = options;
    const text = Text(message);
    const { element: progressBar, setValue } = Progress();
    const dialog = Dialog({ onCancel: abort }, text);

    const set = (state: ProgressState) => {
        if (signal.aborted) {
            return;
        }

        if (state.message) {
            text.innerHTML = state.message.replace(/ /g, "&nbsp;").replace(/\n/g, "<br />");
        }

        if (state.percent !== undefined) {
            setValue(state.percent);
        }
    };

    if (abort) {
        dialog.appendChild(
            Footer(
                progressBar,
                CancelButton()
            )
        );
    } else {
        dialog.appendChild(progressBar);
    }

    document.body.appendChild(dialog);
    let job = fn(set, signal);

    if (listenForAbort) {
        job = Promise.race([job, listenForAbort()]);
    }

    try {
        return await job;
    } finally {
        signal.aborted || closeDialog(dialog, "OK");
    }
}
