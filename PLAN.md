# org-cli Site Pipeline: Architecture Plan

This plan covers issues #1–#14 and resolves the design questions around
where each feature lives, how configuration works, and how to avoid a
tangle of ad-hoc options.

---

## The three-stage mental model

Every issue falls cleanly into one of three stages:

```
[SOURCE .org files]
       │
       ▼
  Stage 1 — Source normalisation   (mutates .org files in-place, safe to re-run)
       │
       ▼
  Stage 2 — Build                  (org → HTML, reads config, emits output dir)
       │
       ▼
  Stage 3 — Output post-processing (mutates HTML in output dir only)
```

Keeping these stages distinct means no single pass has to do everything,
the source files are never corrupted by HTML-level concerns, and the
output dir can always be regenerated from scratch.

---

## Stage 1 — Source normalisation (issues #11, #12, #13)

These operate on `.org` files before any export. They are **idempotent
source fixups** — safe to run repeatedly, never destructive.

| Issue | Operation |
|---|---|
| #11 | Flatten nested `id:` links; strip `\u200B` |
| #12 | Consolidate `:BACKLINKS:` drawer entries |
| #13 | Skip `.#*.org` Emacs lock symlinks during all directory scanning |

**CLI surface**

```
org-cli normalise [--dir <path>] [--dry-run]
```

`--dry-run` prints what would change without writing.
Normalisation also runs automatically as the first step of `org-cli build`.

**Implementation note**
Issue #13 (lock file exclusion) is not a command — it is a invariant
that every directory-walking function in the codebase must enforce.
Add a single shared helper `fn collect_org_files(dir) -> Vec<PathBuf>`
that filters `.#` prefixes, and use it everywhere.

---

## Stage 2 — Build (issues #1, #2, #3, #4, #5, #10, #14)

This is the main `org-cli build` command. It reads a project config
file, runs Stage 1 normalisation, then converts org → HTML.

### 2a. Project config file — `org-cli.toml`

A single `org-cli.toml` in the project root drives the entire build.
No flags proliferate onto the command line.

```toml
[site]
title = "andyreagan.com"           # used as <title> fallback (#10)
base_url = "https://andyreagan.com"
source_dir = "."                   # where .org files live
output_dir = "~/public_html"
static_dirs = ["static", "teaching", "presentations"]
strip_path_prefix = "Library/CloudStorage/SynologyDrive-OnDemandSync/org/"  # (#9)
private_placeholder = "private.html"  # (#4)

[blog]
enabled = true
index_file = "blog.org"            # generated output
date_format = "%B %d, %Y"
nav_random_seed = 42               # deterministic random post (#3)

[scrub]
enabled = true
rules_file = "scrub.toml"          # separate file, see Stage 3
skip_files = ["firefighting.html"] # (#7 per-file opt-out)

[images]
enabled = true
max_width = 1280
max_height = 720
quality = 80
greyscale = true
grain = true
```

**Why a separate `scrub.toml`?**
The scrub rules contain real personal data (real → fake mappings).
Keeping them in a separate file makes it easy to `.gitignore` just that
file while committing everything else.

### 2b. Blog index and tag pages (issues #1, #2)

During the build scan, any `.org` file matching `YYYY-MM-DD-*.org` is
treated as a blog post. The builder:

1. Parses title, tags, and word count from each post.
2. Emits `blog.org` (sorted newest-first) and one `tag_<name>.org` per
   tag into the source directory, then exports them to HTML like any
   other file.

Generating into the source dir (rather than directly to HTML) means
these synthetic files go through the same export pipeline as everything
else — privacy gates, scrubbing, title injection, etc. — for free.

### 2c. Navigation links (issue #3)

During the blog-post scan (same pass as #1/#2), each post is rewritten
in-place to replace any line containing `[random]` with the current
prev/next/random navigation line. This is Stage 1 work triggered by the
build but logically a source normalisation — it modifies `.org` files
idempotently.

Open question resolved: the `[random]` token approach is kept because it
lets the author control placement. A missing token emits a build warning,
not an error, so new posts without it don't break the build.

### 2d. Privacy gates (issues #4, #5)

Both privacy mechanisms are handled inside the exporter (Stage 2), not
as post-processing:

- `#+PRIVATE: true` — detected before parsing; the file is exported as
  a copy of `private_placeholder` and excluded from blog/tag indices.
- `#+BEGIN_PRIVATE` / `#+END_PRIVATE` — stripped from the org AST
  before HTML generation (extends the existing block parsing).

Handling these at the AST level (rather than as HTML post-processing) is
strictly safer: there is no window where sensitive content exists in an
intermediate HTML file.

### 2e. `<title>` injection (issue #10)

Handled natively in the HTML renderer — not post-processing. Priority:
`#+TITLE:` > first heading > `site.title` from config. Since we own the
renderer this is trivial to add and eliminates the need for a sed pass.

### 2f. Incremental rebuild (issue #14)

The build maintains a cache file `.org-cli-cache.json` in the output
directory storing `{ "filename.org": { "mtime": ..., "hash": ... } }`.

On each build:
- Files unchanged since last build are skipped.
- Synthetic files (`blog.org`, `tag_*.org`) are always regenerated
  because they depend on the full post list.
- `org-cli build --force` clears the cache.

---

## Stage 3 — Output post-processing (issues #6, #7, #8, #9)

These run after the HTML files are written to the output directory. They
never touch source `.org` files.

```
org-cli postprocess [--output <path>]
```

Also called automatically at the end of `org-cli build`.

| Issue | Operation | Notes |
|---|---|---|
| #6 | Remove elements with class `hidden`/`PRIVATE` etc. | HTML-aware (use an HTML parser, not regex) |
| #7 | Scrub personal info via substitution rules | Driven by `scrub.toml` |
| #8 | Image pipeline (resize, greyscale, grain) | Shells out to `magick`; graceful skip if absent |
| #9 | Strip absolute path prefix from `href`/`src` | HTML-aware attribute rewriting |

### The `scrub.toml` format

```toml
[[rule]]
category = "address"
real = "97 Buell St"
fake = "103 Campbell Rd"

[[rule]]
category = "phone"
real = "8023553455"
fake = "2484345509"
# phone rules automatically expand to (802) 355-3455 and 802-355-3455 variants

[[rule]]
category = "email"
real = "realandyreagan@gmail.com"
fake = "andyreagan@gmail.com"

[[rule]]
category = "town"
real = "Burlington"
fake = "Essex"
# town rules automatically match all four case variants
```

**Filenames**: scrubbing only applies to HTML *content*, not filenames.
Scrubbing filenames would silently break internal links. If a filename
itself contains sensitive information, rename the source `.org` file
before building.

---

## The unified `build` command

In normal use nobody calls the stages individually:

```
org-cli build [--force] [--config org-cli.toml]
```

Internally this runs:

```
1. collect_org_files()          — skips .# locks (#13)
2. normalise()                  — #11, #12
3. generate_blog_index()        — #1, #2, #3
4. export_all()                 — #4, #5, #10, #14
5. postprocess()                — #6, #7, #8, #9
```

Each step is skipped if its config section is absent or `enabled = false`.

---

## Issue disposition summary

| # | Issue | Stage | Config key |
|---|---|---|---|
| 1 | Blog index generation | 2 — build | `[blog] enabled` |
| 2 | Per-tag index pages | 2 — build | `[blog] enabled` |
| 3 | Prev/next/random navigation | 1 — normalise (during build) | `[blog] nav_random_seed` |
| 4 | `#+PRIVATE: true` gate | 2 — build (AST) | `[site] private_placeholder` |
| 5 | `#+BEGIN_PRIVATE` blocks | 2 — build (AST) | always on |
| 6 | HTML class redaction | 3 — postprocess | always on |
| 7 | Personal info scrubbing | 3 — postprocess | `[scrub]` + `scrub.toml` |
| 8 | Image pipeline | 3 — postprocess | `[images]` |
| 9 | Path prefix rewriting | 3 — postprocess | `[site] strip_path_prefix` |
| 10 | `<title>` injection | 2 — build (renderer) | `[site] title` |
| 11 | Nested id: link flattening | 1 — normalise | always on |
| 12 | Backlinks consolidation | 1 — normalise | always on |
| 13 | Lock file exclusion | shared utility | n/a — invariant |
| 14 | Incremental rebuild cache | 2 — build | `--force` to bypass |

---

## Implementation order

1. **`collect_org_files` utility** — unblocks everything (#13)
2. **`org-cli.toml` config parsing** — needed before any feature lands
3. **`org-cli normalise`** — #11, #12, #13
4. **Blog index + tags + nav** — #1, #2, #3 (one pass, high value)
5. **Privacy gates in exporter** — #4, #5 (AST changes)
6. **`<title>` in renderer** — #10 (small, high polish impact)
7. **Incremental cache** — #14 (makes iteration fast for everything after)
8. **`org-cli postprocess`** — #6, #9 (HTML-aware, no external deps)
9. **Scrubbing** — #7 (`scrub.toml` format + substitution engine)
10. **Image pipeline** — #8 (external dep, nice-to-have)
