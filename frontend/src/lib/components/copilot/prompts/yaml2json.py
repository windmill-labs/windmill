import yaml, json

with open("gen.yaml", "r") as f:
    promptConfig = json.dumps(yaml.load(f, Loader=yaml.CLoader), indent=2)
    with open("genPrompt.ts", "w") as f2:
        f2.write("export const GEN_PROMPT = " + promptConfig + ";")

with open("edit.yaml", "r") as f:
    promptConfig = json.dumps(yaml.load(f, Loader=yaml.CLoader), indent=2)
    with open("editPrompt.ts", "w") as f2:
        f2.write("export const EDIT_PROMPT = " + promptConfig + ";")

with open("fix.yaml", "r") as f:
    promptConfig = json.dumps(yaml.load(f, Loader=yaml.CLoader), indent=2)
    with open("fixPrompt.ts", "w") as f2:
        f2.write("export const FIX_PROMPT = " + promptConfig + ";")
