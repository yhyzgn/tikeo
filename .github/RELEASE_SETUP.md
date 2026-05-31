# GitHub Actions release setup checklist

The repository now has two workflow lanes:

- `CI` (`.github/workflows/ci.yml`): runs on push to `main` and pull requests. It compiles/tests/builds server, web, Java SDK, Rust SDK, and validates Docker image builds without pushing.
- `Release` (`.github/workflows/release.yml`): runs only when pushing a `v*` tag. It uploads GitHub Release assets and pushes Docker Hub images.

## Required repository secrets

Configure these under GitHub repository settings → Secrets and variables → Actions:

| Secret | Required for | Placeholder / example |
| --- | --- | --- |
| `DOCKERHUB_USERNAME` | Tag release Docker push | `your-dockerhub-user-or-org` |
| `DOCKERHUB_TOKEN` | Tag release Docker push | Docker Hub access token with write permission |

`GITHUB_TOKEN` is provided automatically by GitHub Actions for Release asset upload.

## Docker Hub repositories

Create or grant access to these Docker Hub repositories before the first tag release:

- `${DOCKERHUB_USERNAME}/tikee-server`
- `${DOCKERHUB_USERNAME}/tikee-web`

## Tag release procedure

```bash
git tag v0.1.0
git push origin v0.1.0
```

The tag workflow will publish:

- Cross-platform server archives for Linux x86_64, macOS x86_64, macOS arm64, and Windows x86_64.
- Web dist archive with nginx config and Dockerfile.
- Java SDK jar/source jar archive.
- Rust SDK `.crate` archive.
- Docker Hub images tagged with the Git tag and `latest`.

## Notes

- Push/PR workflows intentionally do not publish releases or push images.
- Release publishing is tag-only to avoid accidental external publication from normal development pushes.
