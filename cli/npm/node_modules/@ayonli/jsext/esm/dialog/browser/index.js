import CancelButton from './CancelButton.js';
import Dialog, { closeDialog } from './Dialog.js';
import Footer from './Footer.js';
import Input from './Input.js';
import OkButton from './OkButton.js';
import Progress from './Progress.js';
import Text from './Text.js';

async function alertInBrowser(message) {
    await new Promise(resolve => {
        document.body.appendChild(Dialog({
            onCancel: () => resolve(),
            onOk: () => resolve(),
        }, Text(message), Footer(OkButton())));
    });
}
async function confirmInBrowser(message) {
    return new Promise(resolve => {
        document.body.appendChild(Dialog({
            onCancel: () => resolve(false),
            onOk: () => resolve(true),
        }, Text(message), Footer(CancelButton(), OkButton())));
    });
}
async function promptInBrowser(message, options) {
    const { type, defaultValue } = options;
    return new Promise(resolve => {
        document.body.appendChild(Dialog({
            onCancel: () => resolve(null),
            onOk: (dialog) => {
                const input = dialog.querySelector("input");
                resolve(input.value);
            },
        }, Text(message), Input({ type, value: defaultValue }), Footer(CancelButton(), OkButton())));
    });
}
async function progressInBrowser(message, fn, options) {
    const { signal, abort, listenForAbort } = options;
    const text = Text(message);
    const { element: progressBar, setValue } = Progress();
    const dialog = Dialog({ onCancel: abort }, text);
    const set = (state) => {
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
        dialog.appendChild(Footer(progressBar, CancelButton()));
    }
    else {
        dialog.appendChild(progressBar);
    }
    document.body.appendChild(dialog);
    let job = fn(set, signal);
    if (listenForAbort) {
        job = Promise.race([job, listenForAbort()]);
    }
    try {
        return await job;
    }
    finally {
        signal.aborted || closeDialog(dialog, "OK");
    }
}

export { alertInBrowser, confirmInBrowser, progressInBrowser, promptInBrowser };
//# sourceMappingURL=index.js.map
