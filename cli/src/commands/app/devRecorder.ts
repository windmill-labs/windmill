/**
 * `wmill app dev --recording`: a shell page that frames the app under
 * development and records the session with the same recorder the Windmill UI
 * uses, so a locally-built app can be demoed without deploying it first.
 *
 * The shell holds the toolbar rather than the app page: the recorder snapshots
 * the framed document on every mutation, and a toolbar living in that document
 * would record itself. It also lives at its own path, so the app keeps the root
 * URL its router and its own links expect.
 */
import { DEV_RECORDER_BUNDLE } from "./devRecorderBundle.gen.ts";

/** Path the bundled recorder is served from. */
export const RECORDER_BUNDLE_PATH = "/__wm_recorder.js";
/** Path the toolbar shell is served from. Not the root: the app stays there, so
 * a link or a router push to `/` inside it reloads the app rather than nesting
 * another shell in the frame. */
export const RECORDER_SHELL_PATH = "/__record";
/** Recordings are POSTed here and served back from `<path>/<file>`. */
export const RECORDER_SAVE_PATH = "/__recordings";

export { DEV_RECORDER_BUNDLE };
export { RECORDINGS_FOLDER } from "./app_metadata.ts";

/** Whether a request to the save route came from the shell this server serves.
 * A cross-site POST carrying JSON under a simple content type (`text/plain`)
 * needs no preflight, so without this any page open in the developer's browser
 * could write files into the app folder. Compared against the `Host` the browser
 * actually reached us on, since the shell is equally `localhost`, `127.0.0.1` or
 * whatever `--host` binds. */
export function isOwnOrigin(
  origin: string | undefined,
  host: string | undefined,
): boolean {
  // Absent on a same-origin request in some browsers, and on non-browser
  // clients (curl, a script); a forged one is no worse than no header at all.
  if (origin === undefined) return true;
  if (host === undefined) return false;
  try {
    return new URL(origin).host === host;
  } catch {
    return false;
  }
}

/** Name a recording after the moment it was saved. Millisecond resolution plus
 * the caller's collision retry: two tabs stopping together must not have one
 * silently overwrite the other. */
export function recordingFileName(now: Date, attempt = 0): string {
  const stamp = now.toISOString().slice(0, 23).replace(/[:T.]/g, "-");
  return `recording-${stamp}${attempt > 0 ? `-${attempt}` : ""}.json`;
}

/** Names the read route accepts. Rejects every separator and escape, so the
 * name can only ever resolve inside the recordings folder. */
export function isRecordingFileName(file: string): boolean {
  return /^[A-Za-z0-9._-]+\.json$/.test(file) && !file.includes("..");
}

export function createRecorderShellHTML(opts: {
  appPath: string;
  workspace: string;
  /** Base URL of the Windmill instance, for the "Open in player" link. */
  playerBaseUrl?: string;
}): string {
  // `<` escaped: this lands inside an inline <script>, where a `</script>` in
  // any value (the app path comes from raw_app.yaml) would end the block.
  const config = JSON.stringify({
    appPath: opts.appPath,
    workspace: opts.workspace,
    playerBaseUrl: opts.playerBaseUrl ?? null,
    savePath: RECORDER_SAVE_PATH,
  }).replace(/</g, "\\u003c");
  return `
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Windmill App Dev Recording</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    html, body { height: 100%; }
    body {
      display: flex;
      flex-direction: column;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', sans-serif;
      background: #18181b;
    }
    #wm-rec-bar {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 8px 12px;
      color: #e4e4e7;
      font-size: 13px;
      border-bottom: 1px solid #27272a;
      flex-wrap: wrap;
    }
    #wm-rec-bar button, #wm-rec-bar a.wm-rec-action {
      font: inherit;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 5px 12px;
      border-radius: 6px;
      border: 1px solid #3f3f46;
      background: #27272a;
      color: #e4e4e7;
      cursor: pointer;
      text-decoration: none;
    }
    #wm-rec-bar button:hover, #wm-rec-bar a.wm-rec-action:hover { background: #3f3f46; }
    #wm-rec-bar button:disabled { opacity: 0.5; cursor: not-allowed; }
    #wm-rec-toggle.recording { background: #dc2626; border-color: #dc2626; color: white; }
    #wm-rec-toggle.recording:hover { background: #b91c1c; }
    .wm-rec-dot { width: 8px; height: 8px; border-radius: 50%; background: #dc2626; }
    #wm-rec-toggle.recording .wm-rec-dot { background: white; }
    #wm-rec-status { color: #a1a1aa; }
    #wm-rec-hint { margin-left: auto; color: #71717a; font-size: 12px; }
    #wm-rec-hint code { background: #27272a; padding: 1px 5px; border-radius: 4px; }
    #wm-rec-frame { flex: 1; width: 100%; border: 0; background: white; }
    [hidden] { display: none !important; }
  </style>
</head>
<body>
  <div id="wm-rec-bar">
    <button id="wm-rec-toggle"><span class="wm-rec-dot"></span><span id="wm-rec-toggle-label">Record</span></button>
    <span id="wm-rec-status">Not recording</span>
    <a id="wm-rec-open" class="wm-rec-action" target="_blank" rel="noopener" hidden>Open in player</a>
    <button id="wm-rec-download" hidden>Download JSON</button>
    <span id="wm-rec-hint">Passwords are masked. Mark sensitive elements with <code>data-wm-no-record</code></span>
  </div>
  <iframe id="wm-rec-frame" src="/"></iframe>
  <script src="${RECORDER_BUNDLE_PATH}"></script>
  <script>
    (function () {
      var config = ${config};
      var iframe = document.getElementById('wm-rec-frame');
      var toggle = document.getElementById('wm-rec-toggle');
      var toggleLabel = document.getElementById('wm-rec-toggle-label');
      var status = document.getElementById('wm-rec-status');
      var openLink = document.getElementById('wm-rec-open');
      var downloadBtn = document.getElementById('wm-rec-download');
      var recorder = window.__wmillRecorder.createRawAppRecording();
      var recording = null;
      var ticker = null;

      // The dev-server shim posts both the request and its answer up to the
      // shell; the recorder reads the answer off the framed window, the way the
      // deployed runner delivers it. Relaying is what makes a step wait for the
      // job it launched instead of recording the spinner.
      var pending = [];
      var stranded = [];
      window.addEventListener('message', function (e) {
        var frameWindow = iframe.contentWindow;
        if (!frameWindow || e.source !== frameWindow) return;
        var data = e.data;
        if (!data || typeof data.type !== 'string') return;
        // A fresh document announced itself, so every call the previous one had
        // in flight died with its WebSocket.
        if (data.type === 'wmillDevReady') {
          stranded = stranded.concat(pending);
          pending = [];
          return;
        }
        if (data.reqId === undefined) return;
        if (!/Res$/.test(data.type)) {
          pending.push(data.reqId);
          return;
        }
        pending = pending.filter(function (id) { return id !== data.reqId });
        frameWindow.postMessage(data, window.location.origin);
      });

      // The recorder keeps a reload's outstanding request ids on purpose (a
      // deployed app is answered by its parent, which survives). Here the answer
      // died with the frame's WebSocket, so Stop would wait out the whole job
      // budget unless the shell settles them. Deferred past the load handlers:
      // the recorder rebinds its listener onto the new document in one of them.
      iframe.addEventListener('load', function () {
        if (stranded.length === 0) return;
        var ids = stranded;
        stranded = [];
        setTimeout(function () {
          var frameWindow = iframe.contentWindow;
          if (!frameWindow) return;
          ids.forEach(function (reqId) {
            frameWindow.postMessage({ type: 'unloadRes', reqId: reqId }, window.location.origin);
          });
        }, 0);
      });

      function setStatus(text) { status.textContent = text; }

      function steps(n) { return n + (n === 1 ? ' step' : ' steps'); }

      function tick() {
        // Stop can wait a minute on a runnable the last step launched, so the
        // toolbar has to say what it is waiting for rather than look wedged.
        if (recorder.stopping) setStatus('Waiting for the last job to finish…');
        else if (recorder.active) {
          setStatus('Recording: ' + steps(recorder.stepCount));
        }
      }

      function start() {
        recording = null;
        openLink.hidden = true;
        downloadBtn.hidden = true;
        if (!recorder.start(iframe, { appPath: config.appPath, workspace: config.workspace })) {
          setStatus('Cannot record: the app document is unreachable');
          return;
        }
        toggle.classList.add('recording');
        toggleLabel.textContent = 'Stop';
        tick();
        ticker = setInterval(tick, 250);
      }

      async function stop() {
        toggle.disabled = true;
        setStatus('Finishing…');
        // The ticker outlives the click: it is what reports the drain below.
        recording = await recorder.stop();
        clearInterval(ticker);
        ticker = null;
        toggle.classList.remove('recording');
        toggleLabel.textContent = 'Record';
        downloadBtn.hidden = false;
        setStatus(steps(recording.steps.length) + ' recorded');
        // Only now: starting a fresh recording drops the one being uploaded.
        await save();
        toggle.disabled = false;
      }

      async function save() {
        try {
          var res = await fetch(config.savePath, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(recording)
          });
          var body = await res.json();
          if (!res.ok) throw new Error(body.error || res.statusText);
          setStatus(steps(recording.steps.length) + ' saved to ' + body.file);
          if (config.playerBaseUrl) {
            var src = window.location.origin + config.savePath + '/' + body.file;
            openLink.href = config.playerBaseUrl + 'replay?src=' + encodeURIComponent(src);
            openLink.hidden = false;
          }
        } catch (e) {
          setStatus('Recorded, but saving failed: ' + (e && e.message ? e.message : e));
        }
      }

      toggle.addEventListener('click', function () {
        if (recorder.active) stop();
        else start();
      });

      downloadBtn.addEventListener('click', function () {
        if (recording) recorder.download(recording);
      });
    })();
  </script>
</body>
</html>
`;
}
