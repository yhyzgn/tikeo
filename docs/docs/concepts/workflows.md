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

## Evaluation checklist

A workflow evaluation should prove more than graph rendering. Create or inspect a workflow definition, verify dependency order, trigger a run, inspect instance/attempt records, and confirm replay data contains enough evidence for incident review.

## Failure handling model

Retries, cancellation, and downstream dependency behavior must be explicit. Public docs should avoid vague language like "self-healing" unless the exact retry, backoff, and rollback behavior is implemented and tested.

## Relationship to jobs

Jobs are reusable execution definitions. Workflows compose jobs and workflow-local nodes into a DAG. Topology and impact pages help operators understand which jobs and workflows depend on one another before making changes.

## Future reference work

A later docs phase should include YAML/JSON examples once the schema is generated or copied from verified fixtures. Until then, this page stays conceptual and points readers to the Web UI and API references.
