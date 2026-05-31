# GitHub Actions release setup checklist

The repository uses one validation lane plus independent publish lanes. Normal development pushes never publish external artifacts.

## Workflow lanes

| Workflow | File | Trigger | Publishes |
| --- | --- | --- | --- |
| CI | `.github/workflows/ci.yml` | Push to `main`, pull request | Nothing; validates server, web, Java SDK, Rust SDK, and Docker builds with `push: false`. |
| GitHub assets | `.github/workflows/release-github-assets.yml` | `v*` tag or manual dispatch | Cross-platform server archives and web dist archive. |
| Docker server | `.github/workflows/publish-docker-server.yml` | `v*` tag or manual dispatch | `${DOCKERHUB_USERNAME}/tikee-server`. |
| Docker web | `.github/workflows/publish-docker-web.yml` | `v*` tag or manual dispatch | `${DOCKERHUB_USERNAME}/tikee-web`. |
| Java SDK | `.github/workflows/publish-java-sdk.yml` | `v*` tag or manual dispatch | Java SDK jar/source-jar archive attached to GitHub Release. |
| Rust SDK | `.github/workflows/publish-rust-sdk.yml` | `v*` tag or manual dispatch | Rust SDK `.crate` archive attached to GitHub Release. |

Manual dispatch keeps each target independently releasable. Use the same `v*` tag input as the release version when running a publish workflow manually.

## Required repository secrets

Configure these under GitHub repository settings → Secrets and variables → Actions:

| Secret | Required for | Placeholder / example |
| --- | --- | --- |
| `DOCKERHUB_USERNAME` | Docker server/web publish workflows | `your-dockerhub-user-or-org` |
| `DOCKERHUB_TOKEN` | Docker server/web publish workflows | Docker Hub access token with write permission |

`GITHUB_TOKEN` is provided automatically by GitHub Actions for GitHub Release asset upload.

## Optional future registry-publish placeholders

Current SDK publish workflows attach validated packages to GitHub Releases. If the project later publishes directly to package registries, add dedicated workflows or extend the SDK workflows with these secrets:

| Secret | Future use |
| --- | --- |
| `MAVEN_CENTRAL_USERNAME` | Java SDK Maven Central publish username. |
| `MAVEN_CENTRAL_PASSWORD` | Java SDK Maven Central publish token/password. |
| `MAVEN_SIGNING_KEY` | Java SDK artifact signing key. |
| `MAVEN_SIGNING_PASSWORD` | Java SDK artifact signing password. |
| `CRATES_IO_TOKEN` | Rust SDK crates.io publish token. |

## Docker Hub repositories

Create or grant access to these Docker Hub repositories before the first Docker publish:

- `${DOCKERHUB_USERNAME}/tikee-server`
- `${DOCKERHUB_USERNAME}/tikee-web`

## Tag release procedure

```bash
git tag v0.1.0
git push origin v0.1.0
```

Pushing a `v*` tag starts each independent publish workflow. If only one target needs a retry, rerun that workflow or use its manual dispatch input with the same tag.

## Publish boundaries

- Push/PR workflows intentionally do not publish releases or push images.
- GitHub asset release does not log in to Docker Hub.
- Docker server and Docker web are separate workflows and do not build/push each other.
- Java SDK and Rust SDK packaging are separate workflows and do not depend on each other.
- Add new publish destinations as separate workflows unless they must share a transaction boundary.
