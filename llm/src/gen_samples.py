import argparse
import yaml
from dotenv import load_dotenv
from tqdm import tqdm
from test_data import RESOURCE_TYPES, DB_SCHEMA

load_dotenv()

import openai

import re


from typing import TypedDict, Tuple


class Literal(str):
    pass


def literal_presenter(dumper, data):
    return dumper.represent_scalar("tag:yaml.org,2002:str", data, style="|")


yaml.add_representer(Literal, literal_presenter)


class Prompt(TypedDict):
    prompt: str


class PromptsConfig(TypedDict):
    prompts: dict[str, Prompt]
    system: str


class Query(TypedDict):
    description: str
    type: str
    lang: str
    code: str
    error: str


def get_prompts(
    prompts_path: str,
) -> Tuple[PromptsConfig, PromptsConfig, PromptsConfig]:
    GEN_CONFIG = None
    EDIT_CONFIG = None
    FIX_CONFIG = None
    with open(prompts_path + "/gen.yaml") as f:
        GEN_CONFIG = yaml.safe_load(f)
    with open(prompts_path + "/edit.yaml") as f:
        EDIT_CONFIG = yaml.safe_load(f)
    with open(prompts_path + "/fix.yaml") as f:
        FIX_CONFIG = yaml.safe_load(f)
    return GEN_CONFIG, EDIT_CONFIG, FIX_CONFIG


def get_queries(queries_path: str) -> list[Query]:
    with open(queries_path) as f:
        return yaml.safe_load(f)


def prepare_prompt(
    query: Query,
    GEN_CONFIG: PromptsConfig,
    EDIT_CONFIG: PromptsConfig,
    FIX_CONFIG: PromptsConfig,
):
    system = ""
    prompt = ""
    template_prompt = ""
    if query["type"] == "gen":
        system = GEN_CONFIG["system"]
        template_prompt = GEN_CONFIG["prompts"][query["lang"]]["prompt"]
        prompt = template_prompt.replace("{description}", query["description"])
    elif query["type"] == "edit":
        system = EDIT_CONFIG["system"]
        template_prompt = EDIT_CONFIG["prompts"][query["lang"]]["prompt"]
        prompt = template_prompt.replace("{description}", query["description"]).replace(
            "{code}", query["code"]
        )
    elif query["type"] == "fix":
        system = FIX_CONFIG["system"]
        template_prompt = FIX_CONFIG["prompts"][query["lang"]]["prompt"]
        prompt = template_prompt.replace("{error}", query["error"]).replace(
            "{code}", query["code"]
        )

    if query["lang"] in ["deno", "bun", "nativets"]:
        prompt = prompt.replace("{resourceTypes}", RESOURCE_TYPES["typescript"])
    elif query["lang"] == "python3":
        prompt = prompt.replace("{resourceTypes}", RESOURCE_TYPES["python"])
    elif query["lang"] == "php":
        prompt = prompt.replace("{resourceTypes}", RESOURCE_TYPES["php"])

    if query["lang"] in ["postgresql"]:
        prompt = (
            prompt
            + "\nHere's the database schema, each column is in the format [name, type, required, default?]: <dbschema>\n"
            + DB_SCHEMA
            + "\n</dbschema>"
        )

    return system, prompt, template_prompt


def format_literal(answer: str):
    return re.sub("[^\\S\n]+\n", "\n", answer).replace("\t", "    ")


def gen_samples(queries_path: str, answers_path: str, prompts_path: str):
    GEN_CONFIG, EDIT_CONFIG, FIX_CONFIG = get_prompts(prompts_path)

    queries = get_queries(queries_path)

    answers = []

    for query in tqdm(queries):
        (system, prompt, template_prompt) = prepare_prompt(
            query, GEN_CONFIG, EDIT_CONFIG, FIX_CONFIG
        )
        client = openai.OpenAI()
        chat_completion = client.chat.completions.create(
            model="gpt-4o-2024-05-13",
            messages=[
                {"role": "system", "content": system},
                {"role": "user", "content": prompt},
            ],
            temperature=0,
            max_tokens=2048,
            seed=42,
        )

        answer = {
            **query,
            "answer": Literal(format_literal(chat_completion.choices[0].message.content)),  # type: ignore
            "template_system": Literal(format_literal(system)),
            "template_prompt": Literal(format_literal(template_prompt)),
        }

        if "code" in query:
            answer["code"] = Literal(format_literal(query["code"]))

        answers.append(answer)

    with open(answers_path, "w") as f:
        yaml.dump(answers, f)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Process some integers.")
    parser.add_argument("--queries_path", type=str, default="./sample_queries.yaml")
    parser.add_argument("--answers_path", type=str, default="./sample_answers.yaml")
    parser.add_argument(
        "--prompts_path",
        type=str,
        default="../frontend/src/lib/components/copilot/prompts",
    )
    args = parser.parse_args()
    gen_samples(args.queries_path, args.answers_path, args.prompts_path)
