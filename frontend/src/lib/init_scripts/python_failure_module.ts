export default `import os

def main(message: str, name: str):
    flow_id = os.environ.get("WM_FLOW_JOB_ID")
    print("message", message)
    print("name", name)
    return message, flow_id`
