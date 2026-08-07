import {
  EXIT_PLAN_MODE_TOOL,
  EXIT_PLAN_MODE_TOOL_DESCRIPTION,
  PLAN_MODE_MESSAGES,
  derivePlanTitle,
  exitPlanModeArgs,
} from "../../../../../frontend/src/lib/components/copilot/chat/planMode";
import { createToolDef } from "../../../../../frontend/src/lib/components/copilot/chat/shared";
import type { Tool as ProductionTool } from "../../../../../frontend/src/lib/components/copilot/chat/shared";

/**
 * `exit_plan_mode` over a plain flag, from the schema, description and messages production
 * exports — so a case exercises the real gate and the real wording while the posture lives
 * here rather than on AIChatManager.
 *
 * A case starts in plan mode, which is the posture production offers exactly this one tool
 * in. It resolves immediately, since the eval runners define no `requestConfirmation`: the
 * plan is always approved, and a refused one cannot be expressed.
 */
export function createEvalPlanTools(artifacts: {
  create: (
    sessionId: string,
    input: Record<string, unknown>,
  ) => Promise<{ id: string; name: string }>;
  sessionId: string;
  chatId: string;
}): {
  tools: ProductionTool<{}>[];
  isPlanModeActive: () => boolean;
} {
  let planActive = true;
  return {
    isPlanModeActive: () => planActive,
    // Production offers exactly one plan tool at a time, and these cases start in plan
    // mode, so enter_plan_mode is not among them — offered here it only invites the model
    // to spend a turn entering a posture it is already in.
    tools: [
      {
        def: createToolDef(
          exitPlanModeArgs,
          EXIT_PLAN_MODE_TOOL,
          EXIT_PLAN_MODE_TOOL_DESCRIPTION,
        ),
        // Carries `readonly` for the same reason production does: it is the only way out
        // of the posture, so the gate must not refuse it.
        readonly: true,
        // Production refuses the call once the plan is approved, and a conversation holds
        // one plan document. Without this a second call writes a second `role: plan`
        // artifact, and list_artifacts reports two of them as this conversation's plan.
        validateBeforeConfirmation: () =>
          planActive
            ? undefined
            : {
                label: PLAN_MODE_MESSAGES.exitOutsidePlanModeLabel,
                result: PLAN_MODE_MESSAGES.exitOutsidePlanMode,
              },
        fn: async ({ args }) => {
          const summary = exitPlanModeArgs.safeParse(args).data?.summary;
          if (!summary?.trim()) {
            return PLAN_MODE_MESSAGES.missingSummary;
          }
          planActive = false;
          await artifacts.create(artifacts.sessionId, {
            name: derivePlanTitle(summary),
            content: summary,
            kind: "md",
            role: "plan",
            approved: true,
            chatId: artifacts.chatId,
          });
          return PLAN_MODE_MESSAGES.approvedWithDoc;
        },
      },
    ] as ProductionTool<{}>[],
  };
}
