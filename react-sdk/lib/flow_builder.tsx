import FlowBuilderSvelte from "windmill-components/components/FlowBuilder.svelte";
import React from "react";
import "highlight.js/styles/github.css";

import { reactify } from "svelte-preprocess-react";
import { type Writable } from "svelte/store";

const FlowBuilderReact = reactify(FlowBuilderSvelte);

export function FlowBuilder(props: {
  flowStore: Writable<any>;
  flowStateStore: Writable<any>;
}) {
  return (
    <FlowBuilderReact
      initialPath="u/test/test"
      flowStateStore={props.flowStateStore}
      flowStore={props.flowStore}
      selectedId={undefined}
    ></FlowBuilderReact>
  );
}
