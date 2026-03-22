# org-cli Build Task

Build a Rust CLI for interacting with org-mode files.

## Step 1: Download the org-mode manual

```
mkdir -p docs
curl -s 'https://orgmode.org/org.html' -o docs/org-manual-full.html
```

Read it to understand the spec before writing code.

## Step 2: Write tests FIRST (TDD)

Create comprehensive tests in tests/ covering:
- Parsing headings at any depth (*, **, ***, etc.)
- TODO keywords: TODO, DONE, NEXT, WAITING, CANCELLED, IN-PROGRESS
- Tags: :tag1:tag2:
- Timestamps: active <2026-03-21 Sat>, inactive [2026-03-21 Sat], SCHEDULED:, DEADLINE:, CLOSED:
- Properties drawers: :PROPERTIES: ... :END:
- Priority cookies: [#A], [#B], [#C]
- Links: [[url][description]], [[file:path]]
- Body text under headings
- Multi-file scanning
- Round-trip: parse then re-serialize should produce identical output

## Step 3: Implement the CLI

Only after tests are written, implement these subcommands:

- list — list all TODO/NEXT/WAITING items across files, with file:line, grouped by keyword
- add TEXT --file PATH --tag TAG — append TODO to file under today's daily entry
- done FILE LINE — mark item DONE with CLOSED timestamp
- cancel FILE LINE — mark CANCELLED with CLOSED timestamp
- wait FILE LINE --date YYYY-MM-DD — mark WAITING with optional SCHEDULED date
- reschedule FILE LINE --date YYYY-MM-DD — update SCHEDULED date
- show FILE — pretty-print headings and TODOs in a file

## Tech

- clap for CLI argument parsing
- nom or pest for org parsing
- chrono for dates
- Run `cargo test` after each feature; all tests must pass

## Notes

Andy's org files are on SSH at: andyreagan@100.120.245.106:/var/services/homes/andyreagan/org/
But the CLI should work on any local path too.

## Done signal

When the implementation is complete and all tests pass, run:
openclaw system event --text "Done: org-cli Rust project complete, all tests passing" --mode now
