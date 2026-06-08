---
title: Workflow DAG model
description: How Tikeo models jobs, workflow DAGs, replay, retries, and execution evidence.
---

# Workflow DAG model

Tikeo supports both simple scheduled jobs and workflow definitions. A workflow is a DAG of task nodes with explicit dependencies, dispatch constraints, retry behavior, and execution evidence.

## Current concepts

- **Job**: reusable execution definition.
- **Instance**: one execution of a job or workflow.
- **Attempt**: one run attempt for an instance.
- **Workflow DAG**: nodes and edges for multi-step orchestration.
- **Replay bundle**: data useful for incident review and visual replay.

## UI direction

The Web console includes workflow canvas and topology surfaces. Docs pages should teach the model before diving into JSON/YAML details.

## Safety rule

Workflow docs must distinguish implemented runtime behavior from roadmap items. Do not claim a visual feature or migration tool is complete unless the repository contains verified tests or artifacts.
