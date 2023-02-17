# Release Process

This document captures the steps to follow when releasing a new version of `eksup`. In the future, some or all of these steps may be automated.

1. Create a new tag on `main` with the new version number.

  ```sh
  git tag -a v0.1.0 -m "Release v0.1.0"
  git push origin v0.1.0
  ```

2. This will kick off a GitHub Actions workflow that will publish the GitHub release, and start building the various release artifacts. As the artifacts finish building, they will be attached to the release automatically by the workflow.

3. Once the release is published, update the Homebrew tap formula to point to the new release using the script provided in the tap reposiotry. The tap formula is located at [homebrew-taps](https://github.com/clowdhaus/homebrew-taps).

  ```sh
  ./update_sha256.sh eksup v0.1.0
  git add --a
  git commit -m 'feat: Update eksup to v0.1.0'
  git push origin main
  ```

4. Update the `eksup` documentation site to ensure any changes have been synced with the documetnation. This is done from within the `eksup` repository.

  ```sh
  mkdocs gh-deploy
  ```

5. Update package on crates.io. Update the version of `eksup` used throughout the project as well as within `Cargo.toml`. Commit the changes and push to `main` before publishing the new version to crates.io.

  ```sh
  cargo publish
  ```
