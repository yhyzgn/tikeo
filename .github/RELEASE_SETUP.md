# GitHub Actions release setup checklist

The repository uses one validation lane plus independent publish lanes. Normal development pushes never publish external artifacts.

## Workflow lanes

| Workflow | File | Trigger | Publishes |
| --- | --- | --- | --- |
| CI | `.github/workflows/ci.yml` | Push to `main`, pull request | Nothing; validates server, web, SDKs, demos, deploy tooling, and Docker builds with `push: false`. |
| GitHub assets | `.github/workflows/release-github-assets.yml` | `v*` tag or manual dispatch | Cross-platform server archives, web dist archive, Terraform provider binaries, K8s operator binaries, CRD/manifests, Helm chart, and deploy source package. |
| Docker server | `.github/workflows/publish-docker-server.yml` | `v*` tag or manual dispatch | `yhyzgn/tikeo-server`. |
| Docker web | `.github/workflows/publish-docker-web.yml` | `v*` tag or manual dispatch | `yhyzgn/tikeo-web`. |
| Java SDK | `.github/workflows/publish-java-sdk.yml` | `v*` tag or manual dispatch | Java SDK modules to Maven Central plus GitHub Release tarball. |
| Rust SDK | `.github/workflows/publish-rust-sdk.yml` | `v*` tag or manual dispatch | Rust SDK crate to crates.io plus GitHub Release tarball. |
| Node.js SDK | `.github/workflows/publish-nodejs-sdk.yml` | `v*` tag or manual dispatch | `@yhyzgn/tikeo` to npm plus GitHub Release tarball. |
| Python SDK | `.github/workflows/publish-python-sdk.yml` | `v*` tag or manual dispatch | `tikeo` to PyPI plus GitHub Release tarball. |

Manual dispatch keeps each target independently releasable. Use the same `v*` tag input as the release version when running a publish workflow manually.

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

## Tag release procedure

Use `0.1.xxx` versions for pre-release pipeline integration, for example:

```bash
git tag v0.1.901
git push origin v0.1.901
```

Pushing a `v*` tag starts each independent publish workflow. If a registry publish succeeds and another workflow fails, use the next `0.1.xxx` patch tag for the next full integration pass because public package registries do not allow overwriting an already published version.

## Publish boundaries

- Push/PR workflows intentionally do not publish releases or push images.
- GitHub asset release does not log in to Docker Hub or publish package registries.
- Docker server and Docker web are separate workflows and do not build/push each other.
- SDK publishing workflows are separate by language and can be rerun independently when the target version has not already been published.
- Terraform Provider, K8s operator, CRD, manifest, and Helm chart are currently released as GitHub Release assets only.
- Add new publish destinations as separate workflows unless they must share a transaction boundary.
