from prefect import flow, task

@task
def step_a():
    return 1

@task
def step_b():
    return 2

@task
def step_c():
    return 3

@flow(name="benchmark-3step")
def benchmark_flow():
    a = step_a()
    b = step_b()
    c = step_c()
    return {"a": a, "b": b, "c": c}

if __name__ == "__main__":
    benchmark_flow.serve(name="benchmark-deployment")
