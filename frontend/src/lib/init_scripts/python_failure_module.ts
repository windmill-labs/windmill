export default `import os

def main(message: str, name: str, step_id: str):
    flow_id = os.environ.get("WM_ROOT_FLOW_JOB_ID")
    print("message", message)
    print("name", name)
    print("step_id", step_id)
    return { "message": message, "flow_id": flow_id, "step_id": step_id, "recover": False }`
