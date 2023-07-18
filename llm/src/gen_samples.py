import argparse
import yaml
from dotenv import load_dotenv
from tqdm import tqdm
from resources import RESOURCE_TYPES

load_dotenv()

import openai

import re


from typing import TypedDict, Tuple


class Literal(str):
    pass


def literal_presenter(dumper, data):
    return dumper.represent_scalar("tag:yaml.org,2002:str", data, style="|")


yaml.add_representer(Literal, literal_presenter)


class GenPrompt(TypedDict):
    prompt: str


class GenConfig(TypedDict):
    prompts: dict[str, GenPrompt]
    system: str


class CommonConfig(TypedDict):
    system: str
    prompt: str


class Query(TypedDict):
    description: str
    type: str
    lang: str
    code: str
    error: str


def scriptLangToCodeLang(lang: str):
    if lang in ["deno", "bun", "nativets"]:
        return "typescript"
    elif lang in ["postgresql", "mysql"]:
        return "sql"
    elif lang == "python3":
        return "python"
    elif lang == "bash":
        return "shell"
    elif lang == "frontend":
        return "javascript"
    else:
        return lang


def scriptLangToEnvironment(lang: str):
    if lang == "deno":
        return "typescript in a deno running environment"
    elif lang == "bun":
        return "typescript in a node.js running environment"
    elif lang == "nativets":
        return "typescript where you should use fetch and are not allowed to import any libraries"
    elif lang == "frontend":
        return "client-side javascript"
    else:
        return lang


def get_prompts(prompts_path: str) -> Tuple[GenConfig, CommonConfig, CommonConfig]:
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
    GEN_CONFIG: GenConfig,
    EDIT_CONFIG: CommonConfig,
    FIX_CONFIG: CommonConfig,
):
    system = ""
    prompt = ""
    if query["type"] == "gen":
        system = GEN_CONFIG["system"]
        prompt = GEN_CONFIG["prompts"][query["lang"]]["prompt"]
        prompt = prompt.replace("{description}", query["description"])
        if query["lang"] in ["deno", "bun", "nativets"]:
            prompt = prompt.replace("{resourceTypes}", RESOURCE_TYPES["typescript"])
        elif query["lang"] in ["python3"]:
            prompt = prompt.replace("{resourceTypes}", RESOURCE_TYPES["python"])
    elif query["type"] == "edit":
        system = EDIT_CONFIG["system"]
        prompt = EDIT_CONFIG["prompt"]
        lang = scriptLangToCodeLang(query["lang"])
        environment = scriptLangToEnvironment(query["lang"])
        prompt = (
            prompt.replace("{description}", query["description"])
            .replace("{lang}", lang)
            .replace("{environment}", environment)
            .replace("{code}", query["code"])
        )
    elif query["type"] == "fix":
        system = FIX_CONFIG["system"]
        prompt = FIX_CONFIG["prompt"]
        lang = scriptLangToCodeLang(query["lang"])
        environment = scriptLangToEnvironment(query["lang"])
        prompt = (
            prompt.replace("{lang}", lang)
            .replace("{environment}", environment)
            .replace("{error}", query["error"])
            .replace("{code}", query["code"])
        )

    return system, prompt


def format_answer(answer: str):
    return re.sub("[^\\S\n]+\n", "\n", answer).replace("\t", "    ")


def gen_samples(queries_path: str, answers_path: str, prompts_path: str):
    GEN_CONFIG, EDIT_CONFIG, FIX_CONFIG = get_prompts(prompts_path)

    queries = get_queries(queries_path)

    answers = []
    for query in tqdm(queries):
        system, prompt = prepare_prompt(query, GEN_CONFIG, EDIT_CONFIG, FIX_CONFIG)
        chat_completion = openai.ChatCompletion.create(
            model="gpt-4",
            messages=[
                {"role": "system", "content": system},
                {"role": "user", "content": prompt},
            ],
            temperature=0.5,
            max_tokens=2048,
        )

        answer = {
            **query,
            "answer": Literal(format_answer(chat_completion["choices"][0]["message"]["content"])),  # type: ignore
        }

        if "code" in query:
            answer["code"] = Literal(query["code"])

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
        default="../frontend/src/lib/components/codeGen/prompts",
    )
    args = parser.parse_args()
    gen_samples(args.queries_path, args.answers_path, args.prompts_path)
