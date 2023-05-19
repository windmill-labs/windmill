import AppPreviewSvelte from "windmill-components/components/apps/editor/AppPreview.svelte";
import React from "react";

import { reactify } from "svelte-preprocess-react";
import { App } from "windmill-components/components/apps/types";
import { writable } from "svelte/store";

const AppPreviewReact = reactify(AppPreviewSvelte);
const breakpoint = writable<"lg" | "sm">("lg");
const noBackend = false;

export function AppPreview(props: {
  app: App;
  appPath: string;
  username: string;
  email: string;
  summary: string;
  workspace: string;
}) {
  return (
    <AppPreviewReact
      app={props.app}
      appPath={props.appPath}
      breakpoint={breakpoint}
      policy={{}}
      workspace={props.workspace}
      isEditor={false}
      context={{
        username: props.username ?? "anonymous",
        email: props.email ?? "anonymous",
      }}
      summary={props.summary}
      noBackend={noBackend}
    ></AppPreviewReact>
  );
}
