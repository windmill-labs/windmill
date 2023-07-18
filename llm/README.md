# Test sample queries of the AI feature

## Setup

Create a virtual environment `python -m venv .venv`, activate it `source .venv/bin/activate` and install the requirements `pip install -r requirements.txt`. Put your `OPENAI_API_KEY` in `.env`.

## Run

Run the script `python src/gen_samples.py` and check the results in `sample_answers.yaml`.

## Add queries

You can add more queries in `sample_queries.yaml`. There are three types of queries:

### Generate code from a prompt

```yaml
- type: gen
  description: hello world
  lang: python3
```

### Modifiy code according to a prompt

```yaml
- type: edit
  description: comment
  lang: python3
  code: |-
    print("hello world")
```

### Fix code given an error

```yaml
- type: fix
  lang: python3
  code: |-
    def main():
        return 3 / 0
  error: division by zero
```
