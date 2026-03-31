# AI Evals

This directory contains the shared benchmark data and supporting assets for
black-box AI output evaluation across frontend chat modes and the CLI.

## Purpose

The goal is to evaluate the final artifact produced by the AI system, not the
internal implementation details of the frontend or CLI.

## Layout

- `cases/`: benchmark case manifests
- `fixtures/`: shared reusable starting states and assets
- `history/`: git-tracked benchmark summaries and chart rollups
- `scripts/`: benchmark-history and reporting utilities
- `variants/`: prompt or skill-bundle candidates
- `results/`: run outputs and reports (intended to stay untracked)

## Initial Scope

The first implementation step is to move the existing frontend flow and app
benchmark prompts out of inline test code and into shared case manifests.

Later phases should add:

- frontend `script` cases
- CLI artifact-evaluation cases
- shared result schemas
- repeated-run reliability reporting
- official benchmark history snapshots and rollups
