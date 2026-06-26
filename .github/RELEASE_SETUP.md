# GitHub Actions release setup checklist

The repository uses one validation lane plus independent publish lanes. Normal development pushes never publish external artifacts.

## Workflow lanes

| Workflow | File | Trigger | Publishes | Deployment environment |
| --- | --- | --- | --- | --- |
| CI | `.github/workflows/ci.yml` | Push to `main`, pull request | Nothing; validates server, web, SDKs, demos, deploy tooling, and Docker builds with `push: false`. | none |
| GitHub assets | `.github/workflows/release-github-assets.yml` | `v*` tag or manual dispatch | Cross-platform server binaries, cross-platform `tikeo-migrate` migration CLI binaries, web dist archive, Terraform provider binaries, K8s operator binaries, CRD/manifests, Helm chart, and deploy source package. | `github-release` |
| Migration CLI binary CI | `.github/workflows/build-migrate-cli.yml` | Push/PR touching migration CLI paths or manual dispatch | Nothing external; builds and uploads workflow artifacts for Linux x86_64/arm64, macOS Intel/Apple Silicon, and Windows x86_64/arm64. Release attachment is handled by GitHub assets workflow. | none |
| Release candidate Worker soak | `.github/workflows/release-candidate-worker-soak.yml` | Manual dispatch | Nothing external; runs the cross-language Worker soak for a selected ref and uploads the `cross-language-worker-soak` evidence artifact. | none |
| Docker server | `.github/workflows/publish-docker-server.yml` | `v*` tag or manual dispatch | `yhyzgn/tikeo-server`. | `dockerhub-tikeo-server` |
| Docker web | `.github/workflows/publish-docker-web.yml` | `v*` tag or manual dispatch | `yhyzgn/tikeo-web`. | `dockerhub-tikeo-web` |
| Docker docs | `.github/workflows/publish-docker-docs.yml` | `v*` tag or manual dispatch | `yhyzgn/tikeo-docs`. | `dockerhub-tikeo-docs` |
| Java SDK | `.github/workflows/publish-java-sdk.yml` | `v*` tag or manual dispatch | Java SDK modules to Maven Central plus GitHub Release tarball. | `maven-central-net-tikeo` |
| Rust SDK | `.github/workflows/publish-rust-sdk.yml` | `v*` tag or manual dispatch | Rust SDK crate to crates.io plus GitHub Release tarball. | `crates-io-tikeo` |
| Node.js SDK | `.github/workflows/publish-nodejs-sdk.yml` | `v*` tag or manual dispatch | `@yhyzgn/tikeo` to npm plus GitHub Release tarball. | `npm-yhyzgn-tikeo` |
| Python SDK | `.github/workflows/publish-python-sdk.yml` | `v*` tag or manual dispatch | `tikeo` to PyPI plus GitHub Release tarball. | `pypi-tikeo` |

Manual dispatch keeps each target independently releasable. Use the same `v*` tag input as the release version when running a publish workflow manually.

## GitHub Deployments page

Release publish workflows intentionally attach a GitHub Actions `environment` to every external publishing target. These environments are not fake runtime clusters: they are the real artifact distribution surfaces operators consume after a release tag is cut.

| Environment | Real target represented |
| --- | --- |
| `github-release` | GitHub Release assets: binaries, web dist, Helm chart, manifests, Compose files, and deploy source bundles. |
| `dockerhub-tikeo-server` | Docker Hub `yhyzgn/tikeo-server` image tags. |
| `dockerhub-tikeo-web` | Docker Hub `yhyzgn/tikeo-web` image tags. |
| `dockerhub-tikeo-docs` | Docker Hub `yhyzgn/tikeo-docs` image tags. |
| `crates-io-tikeo` | crates.io `tikeo` Rust SDK crate. |
| `npm-yhyzgn-tikeo` | npm `@yhyzgn/tikeo` Node.js SDK package. |
| `pypi-tikeo` | PyPI `tikeo` Python SDK package. |
| `go-module-tikeo` | Go module tag and Go proxy indexing for `github.com/yhyzgn/tikeo/sdks/go/tikeo`. |
| `maven-central-net-tikeo` | Maven Central `net.tikeo` Java SDK artifacts. |

The GitHub project **Deployments** page is therefore the release distribution ledger: each successful publish job creates a deployment status for the tag and links to the target registry or release page. Validation-only workflows must not use an environment because they do not publish an external artifact.


## Release candidate soak gate

Before cutting a release tag, run **Release candidate / cross-language Worker soak** manually from GitHub Actions. Use the candidate branch or tag as `ref`; keep the default `soak_seconds=120` for a quick release-candidate gate, or increase it for a longer handoff run. The workflow does not publish anything. It uploads `cross-language-worker-soak` with `*-soak-summary.json`, `*-soak-summary.csv`, and `*-soak-metrics.jsonl`, and writes the key verdict numbers to the GitHub step summary.

## Required repository secrets

Configure these under GitHub repository settings → Secrets and variables → Actions:

| Secret | Required for | Placeholder / example |
| --- | --- | --- |
| `DOCKERHUB_USERNAME` | Docker server/web publish workflows | `yhyzgn` |
| `DOCKERHUB_TOKEN` | Docker Hub access token with write permission | Docker Hub token |
| `NPM_TOKEN` | Node.js SDK publish workflow | npm automation token for `@yhyzgn/tikeo` |
| `PYPI_API_TOKEN` | Python SDK publish workflow | PyPI token for `tikeo` |
| `CRATES_IO_TOKEN` | Rust SDK publish workflow | crates.io token for `tikeo` |
| `MAVEN_CENTRAL_USERNAME` | Java SDK Maven Central publish | Central Portal user token username |
| `MAVEN_CENTRAL_PASSWORD` | Java SDK Maven Central publish | Central Portal user token password |
| `MAVEN_SIGNING_KEY` | Java SDK artifact signing | ASCII-armored private GPG key |
| `MAVEN_SIGNING_KEY_ID` | Java SDK artifact signing | GPG key id |
| `MAVEN_SIGNING_PASSWORD` | Java SDK artifact signing | GPG private key passphrase |

`GITHUB_TOKEN` is provided automatically by GitHub Actions for GitHub Release asset upload.

## Docker Hub repositories

Create or grant access to these Docker Hub repositories before the first Docker publish:

- `yhyzgn/tikeo-server`
- `yhyzgn/tikeo-web`
- `yhyzgn/tikeo-docs`

## Tag release procedure

Use `0.1.xxx` versions for pre-release pipeline integration, for example:

```bash
git tag v0.1.901
git push origin v0.1.901
```

Pushing a `v*` tag starts each independent publish workflow. If a registry publish succeeds and another workflow fails, use the next `0.1.xxx` patch tag for the next full integration pass because public package registries do not allow overwriting an already published version.


## GitHub Release notes

GitHub Release body text is generated automatically by `scripts/generate-release-notes.py` inside `.github/workflows/release-github-assets.yml`. The generator is intentionally product-facing rather than a raw commit dump:

- it finds the previous `v*` tag and reads the commit range for the new release;
- it classifies commits by changed paths and subject keywords into release experience, migration toolkit, server/scheduling, web console, SDKs/workers, deployment/operations, documentation, and CI/quality gates;
- it renders `Highlights`, `Downloads`, `Added`, `Changed`, `Fixed`, `Upgrade notes`, `Verification`, and a compact `Commit audit`;
- it builds the download table from the actual staged assets, including raw server binaries, raw migration CLI binaries, web dist, Helm, Docker Compose, Kubernetes manifests, operator, Terraform provider, SDK source packages, and deploy source bundles.

No handwritten release-note file is required. If the generated text looks too mechanical, improve the generator rules and its tests instead of editing a single release by hand.

## Publish boundaries

- Push/PR workflows intentionally do not publish releases or push images.
- GitHub asset release does not log in to Docker Hub or publish package registries.
- Docker server, Docker web, and Docker docs are separate workflows and do not build/push each other.
- SDK publishing workflows are separate by language and can be rerun independently when the target version has not already been published.
- `tikeo-migrate` is built as raw GitHub Release binaries for Linux x86_64/arm64, macOS Intel/Apple Silicon, and Windows x86_64/arm64 so users can download a ready-to-run migration binary without installing Rust or extracting an archive.
- Terraform Provider, K8s operator, CRD, manifest, and Helm chart are currently released as GitHub Release assets only.
- Add new publish destinations as separate workflows unless they must share a transaction boundary.
