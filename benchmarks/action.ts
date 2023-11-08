import {
  FlowValue,
  JobService,
  Preview as ScriptPreview,
} from "https://deno.land/x/windmill@v1.167.0/windmill-api/index.ts";

export type Action = FlowAction | ScriptAction | RandomAction;

export type RandomAction = {
  type: "RANDOM";
  actions: {
    action: Action;
    weight: number;
  }[];
};

export type FlowAction = PreviewFlowAction | PathFlowAction;
export type PreviewFlowAction = {
  type: "PREVIEW_FLOW";
  value: FlowValue;
  args: Record<string, any>;
  workspace: string;
};
export type PathFlowAction = {
  type: "PATH_FLOW";
  path: string;
  workspace: string;
  args: Record<string, any>;
};

export type ScriptAction = PreviewScriptAction | HashScriptAction;
export type PreviewScriptAction = {
  type: "PREVIEW_SCRIPT";
  content: string;
  language: ScriptPreview.language;
  args: Record<string, any>;
  workspace: string;
};
export type HashScriptAction = {
  type: "HASH_SCRIPT";
  hash: string;
  workspace: string;
  args: Record<string, any>;
};

export function evaluate(action: Action): Promise<void> {
  if (action.type == "HASH_SCRIPT") {
    return evaluateHashScript(action);
  } else if (action.type == "PATH_FLOW") {
    return evaluatePathFlow(action);
  } else if (action.type == "PREVIEW_FLOW") {
    return evaluatePreviewFlow(action);
  } else if (action.type == "PREVIEW_SCRIPT") {
    return evaluatePreviewScript(action);
  } else if (action.type == "RANDOM") {
    return evaluateRandom(action);
  } else {
    throw new Error("UNHANDLED ACTION TYPE!!");
  }
}

export function evaluateRandom(action: RandomAction): Promise<void> {
  if (action.actions.length < 1) {
    console.log("empty random action");
    return Promise.resolve();
  } else if (action.actions.length == 1) {
    return evaluate(action.actions[0].action);
  } else {
    const sequentialWeightedActions: { weight: number; action: Action }[] =
      new Array(action.actions.length);
    let total_weight = 0;
    for (let i = 0; i < action.actions.length; i++) {
      sequentialWeightedActions[i] = {
        weight: total_weight + action.actions[i].weight,
        action: action.actions[i].action,
      };
      total_weight += action.actions[i].weight;
    }
    const r = Math.random() * total_weight;
    const selectedAction = sequentialWeightedActions.find(
      (x) => x.weight >= r
    )!.action;

    return evaluate(selectedAction);
  }
}

async function evaluatePreviewScript(
  action: PreviewScriptAction
): Promise<void> {
  await JobService.runScriptPreview({
    workspace: action.workspace,
    requestBody: {
      content: action.content,
      language: action.language,
      args: action.args,
    },
  });
}

async function evaluateHashScript(action: HashScriptAction): Promise<void> {
  await JobService.runScriptByHash({
    workspace: action.workspace,
    hash: action.hash,
    requestBody: action.args,
  });
}

async function evaluatePreviewFlow(action: PreviewFlowAction): Promise<void> {
  await JobService.runFlowPreview({
    workspace: action.workspace,
    requestBody: {
      args: action.args,
      value: action.value,
    },
  });
}

async function evaluatePathFlow(action: PathFlowAction): Promise<void> {
  await JobService.runFlowByPath({
    workspace: action.workspace,
    path: action.path,
    requestBody: action.args,
  });
}
