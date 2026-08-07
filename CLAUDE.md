# Before committing

This repo deploys to GitHub Pages from the committed `docs/` folder (see
README's "Building for the web" section) — Pages serves whatever is
already committed there, not the source. `docs/` is a build artifact, not
generated on push, so it goes stale the moment `src/` or `assets/` change
without a rebuild.

Before every commit that touches `src/` or `assets/`:

1. `cargo build` — fast native sanity check.
2. `make pages` — rebuilds the wasm target *and* regenerates `docs/`
   (this already runs `make web` internally; no separate step needed).
3. `git add` the source changes together with the regenerated `docs/`
   files in the **same commit** — never split them across commits or
   leave `docs/` for later.

Skip step 2 only for changes that can't affect the built app (docs/comments
in files outside `src`/`assets`, this file, etc.).
