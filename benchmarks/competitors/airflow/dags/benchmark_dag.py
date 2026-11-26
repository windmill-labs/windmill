from airflow import DAG
from airflow.operators.python import PythonOperator
from datetime import datetime


def step_a():
    return 1


def step_b():
    return 2


def step_c():
    return 3


with DAG(
    dag_id="benchmark_3step",
    start_date=datetime(2024, 1, 1),
    schedule=None,
    catchup=False,
    max_active_runs=200,
) as dag:
    t1 = PythonOperator(task_id="step_a", python_callable=step_a)
    t2 = PythonOperator(task_id="step_b", python_callable=step_b)
    t3 = PythonOperator(task_id="step_c", python_callable=step_c)

    t1 >> t2 >> t3
