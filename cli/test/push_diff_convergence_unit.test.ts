import { afterAll, beforeAll, expect, test } from "bun:test";
import JSZip from "jszip";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, sep } from "node:path";
import {
  checkoutInlineNames,
  compareDynFSElement,
  ZipFSElement,
} from "../src/commands/sync/sync.ts";

// The differ also reads the working tree (shared lockfiles, dependency files);
// an empty one keeps that out of the picture.
const originalCwd = process.cwd();
beforeAll(() => {
  process.chdir(mkdtempSync(join(tmpdir(), "wmill-push-diff-")));
});
afterAll(() => {
  process.chdir(originalCwd);
});

// A push is only useful when a second run of it finds nothing left to do.
// These pin the two shapes that used to be listed on every run of a push into
// a fork while the push itself either applied nothing or aborted.

type Mock = {
  isDirectory: boolean;
  path: string;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<Mock>;
};

// Both sides of the differ use the OS separator; fixtures are written with
// "/" and rows are read back the same way.
const osPath = (p: string) => p.split("/").join(sep);
const slashPath = (p: string) => p.split(sep).join("/");

function local(files: Record<string, string>): Mock {
  return {
    isDirectory: true,
    path: "",
    async getContentText() {
      return "";
    },
    async *getChildren() {
      for (const [path, content] of Object.entries(files)) {
        yield {
          isDirectory: false,
          path: osPath(path),
          async getContentText() {
            return content;
          },
          async *getChildren() {},
        };
      }
    },
  };
}

const noIgnore = () => false;

async function diff(
  localEl: Mock,
  remoteEl: Mock,
  skips: Record<string, unknown>,
  parentOwnsScheduleEnabled?: (scheduleFilePath: string) => boolean,
) {
  const { changes } = await compareDynFSElement(
    localEl as any,
    remoteEl as any,
    noIgnore,
    false,
    skips as any,
    true,
    [],
    false,
    undefined,
    undefined,
    false,
    false,
    parentOwnsScheduleEnabled,
  );
  return changes.map((c) => `${c.name} ${slashPath(c.path)}`);
}

const SCHEDULE = (enabled: string) =>
  `summary: nightly\nargs: {}\nenabled: ${enabled}\nis_flow: true\nschedule: 0 0 0 * * *\nscript_path: f/mail/flow\ntimezone: UTC\n`;

test("push into a fork: a schedule the parent also has compares without `enabled`", async () => {
  const remote = local({ "f/mail/nightly.schedule.yaml": SCHEDULE("true") });
  const skips = { includeSchedules: true };
  // The parent has `f/mail/nightly`; any other schedule is the fork's own.
  const parentHas = (filePath: string) =>
    slashPath(filePath) === "f/mail/nightly.schedule.yaml";

  // Not a fork: `enabled` is compared like any other field.
  expect(
    await diff(
      local({ "f/mail/nightly.schedule.yaml": SCHEDULE("false") }),
      remote,
      skips,
    ),
  ).toEqual(["edited f/mail/nightly.schedule.yaml"]);
  expect(
    await diff(
      local({ "f/mail/nightly.schedule.yaml": SCHEDULE("false") }),
      remote,
      skips,
      parentHas,
    ),
  ).toEqual([]);
  // The key being absent is the same case as it differing.
  expect(
    await diff(
      local({
        "f/mail/nightly.schedule.yaml": SCHEDULE("false").replace(
          "enabled: false\n",
          "",
        ),
      }),
      remote,
      skips,
      parentHas,
    ),
  ).toEqual([]);
  // Only `enabled` is set aside.
  expect(
    await diff(
      local({
        "f/mail/nightly.schedule.yaml": SCHEDULE("false").replace(
          "0 0 0 * * *",
          "0 0 1 * * *",
        ),
      }),
      remote,
      skips,
      parentHas,
    ),
  ).toEqual(["edited f/mail/nightly.schedule.yaml"]);
  // A schedule only the fork has keeps toggling from the file.
  expect(
    await diff(
      local({ "f/mail/fork_only.schedule.yaml": SCHEDULE("true") }),
      local({ "f/mail/fork_only.schedule.yaml": SCHEDULE("false") }),
      skips,
      parentHas,
    ),
  ).toEqual(["edited f/mail/fork_only.schedule.yaml"]);
});

const SUMMARY = "process one mail end-to-end (spam check, classify)";

function remoteFlow(content: string) {
  const zip = new JSZip();
  zip.file(
    "f/mail/flow_v2.flow.json",
    JSON.stringify({
      summary: "Flow V2",
      description: "",
      value: {
        modules: [
          {
            id: "a",
            summary: SUMMARY,
            value: {
              type: "rawscript",
              content,
              input_transforms: {},
              language: "python3",
            },
          },
        ],
      },
      schema: { type: "object", properties: {} },
    }),
    // The backend's archive carries no directory entries.
    { createFolders: false },
  );
  return zip;
}

function localFlow(content: string) {
  return local({
    "f/mail/flow_v2.flow/flow.yaml": `summary: Flow V2\ndescription: ''\nvalue:\n  modules:\n    - id: a\n      summary: ${SUMMARY}\n      value:\n        type: rawscript\n        content: '!inline process_mail.inline_script.py'\n        input_transforms: {}\n        language: python3\nschema:\n  type: object\n  properties: {}\n`,
    "f/mail/flow_v2.flow/process_mail.inline_script.py": content,
  });
}

// The checkout's `!inline` references, as `push` reads them from its flow.yaml.
const checkoutNames = async (flowDir: string) =>
  slashPath(flowDir) === "f/mail/flow_v2.flow"
    ? { a: "process_mail.inline_script.py" }
    : {};

test("push: an inline script the checkout names differently from the step summary is not a rename", async () => {
  const skips = { includeSchedules: false };
  const render = (content: string, withCheckout: boolean) =>
    ZipFSElement(
      remoteFlow(content),
      true,
      "bun",
      {},
      {},
      false,
      true,
      withCheckout ? checkoutNames : undefined,
    ) as any;

  // Same content, file named by hand: three rows before, none after.
  expect(
    await diff(
      localFlow("def main():\n    return 1\n"),
      render("def main():\n    return 1\n", false),
      skips,
    ),
  ).toEqual([
    "deleted f/mail/flow_v2.flow/process_one_mail_end-to-end_(spam_check,_classify).inline_script.py",
    "edited f/mail/flow_v2.flow/flow.yaml",
    "added f/mail/flow_v2.flow/process_mail.inline_script.py",
  ]);
  expect(
    await diff(
      localFlow("def main():\n    return 1\n"),
      render("def main():\n    return 1\n", true),
      skips,
    ),
  ).toEqual([]);

  // A real edit is still one.
  expect(
    await diff(
      localFlow("def main():\n    return 2\n"),
      render("def main():\n    return 1\n", true),
      skips,
    ),
  ).toEqual(["edited f/mail/flow_v2.flow/process_mail.inline_script.py"]);
});

test("push: a checkout name that collides with another step's summary-derived name keeps two files", async () => {
  const zip = new JSZip();
  zip.file(
    "f/mail/flow_v2.flow.json",
    JSON.stringify({
      summary: "Flow V2",
      description: "",
      value: {
        modules: [
          {
            id: "a",
            summary: SUMMARY,
            value: {
              type: "rawscript",
              content: "a",
              input_transforms: {},
              language: "python3",
            },
          },
          {
            id: "b",
            summary: "process_mail",
            value: {
              type: "rawscript",
              content: "b",
              input_transforms: {},
              language: "python3",
            },
          },
        ],
      },
      schema: { type: "object", properties: {} },
    }),
    { createFolders: false },
  );
  // Nothing local: every rendered file is a "deleted" row, one per path.
  const rows = await diff(
    local({}),
    ZipFSElement(zip, true, "bun", {}, {}, false, true, checkoutNames) as any,
    { includeSchedules: false },
  );
  expect(rows.filter((r) => r.endsWith(".py")).sort()).toEqual([
    "deleted f/mail/flow_v2.flow/process_mail.inline_script.py",
    "deleted f/mail/flow_v2.flow/process_one_mail_end-to-end_(spam_check,_classify).inline_script.py",
  ]);
});

test("push: checkout inline names stay inside the flow folder", async () => {
  const flowYaml = join(process.cwd(), "flow.yaml");
  writeFileSync(
    flowYaml,
    `summary: x\nvalue:\n  modules:\n    - id: a\n      value:\n        type: rawscript\n        content: '!inline a.inline_script.py'\n        language: python3\n    - id: b\n      value:\n        type: rawscript\n        content: '!inline ../shared/b.py'\n        language: python3\n    - id: c\n      value:\n        type: rawscript\n        content: '!inline /tmp/c.py'\n        language: python3\n`,
  );
  expect(await checkoutInlineNames(flowYaml)).toEqual({
    a: "a.inline_script.py",
  });
  expect(
    await checkoutInlineNames(join(process.cwd(), "missing.yaml")),
  ).toEqual({});
});
