# org-cli HTML Export: Feature Gap Analysis & Plan

## Reference: Org Manual v9.8 (downloaded to `references/org-manual-full.html`)

---

## Current State (163 tests passing)

### What We Have
- **Headlines**: Stars at any depth, TODO keywords (6 types), tags, priorities A/B/C
- **Timestamps**: Active/inactive, SCHEDULED/DEADLINE/CLOSED, time ranges, repeaters
- **Properties drawers**: `:PROPERTIES:` / `:END:` with key-value pairs
- **Links**: `[[url][desc]]`, `[[url]]` format; `id:` prefix detection
- **Preamble**: Stored as raw text; `#+TITLE` and `#+AUTHOR` extracted
- **Body text**: Plain text after headings
- **Inline markup**: `*bold*`, `/italic/`, `~code~`, `=verbatim=`, `+strikethrough+`, `_underline_`
- **HTML export**: Full page rendering, CSS, id link resolution, site export with index.html
- **CLI**: `list`, `add`, `done`, `cancel`, `wait`, `reschedule`, `show`, `export`

---

## Gaps (Org features NOT yet handled, ordered by importance for HTML export)

### Priority 1 — Critical for usable HTML export

1. **Plain Lists** (§2.6)
   - Unordered lists: lines starting with `- `, `+ `, or `* ` (when not at column 0)
   - Ordered lists: `1.`, `1)`, etc.
   - Description lists: `- term :: description`
   - Nested lists (indentation-based)
   - Checkboxes: `[ ]`, `[X]`, `[-]`
   - **Currently**: Lists end up as flat body text `<p>` elements

2. **Source Blocks / Literal Examples** (§12.6)
   - `#+BEGIN_SRC lang` ... `#+END_SRC`
   - `#+BEGIN_EXAMPLE` ... `#+END_EXAMPLE`
   - Fixed-width lines: `: prefixed text`
   - Line numbering (`-n`, `+n`)
   - **Currently**: Treated as body text, no `<pre>/<code>` rendering

3. **Tables** (§3, §13.9.9)
   - Org table syntax: `| col1 | col2 |`
   - Header row separator: `|---+---|`
   - Column alignment
   - Captions via `#+CAPTION:`
   - **Currently**: Not parsed at all — rendered as plain text

4. **Block Elements** (§2.8, §12.1)
   - `#+BEGIN_QUOTE` ... `#+END_QUOTE` → `<blockquote>`
   - `#+BEGIN_VERSE` ... `#+END_VERSE` → preserve line breaks
   - `#+BEGIN_CENTER` ... `#+END_CENTER` → centered text
   - `#+BEGIN_EXPORT html` ... `#+END_EXPORT` → raw HTML passthrough
   - Generic `#+BEGIN_xyz` ... `#+END_xyz` → `<div class="xyz">`
   - **Currently**: Not parsed

5. **Footnotes** (§12.10)
   - References: `[fn:1]`, `[fn:name]`, `[fn:: inline def]`, `[fn:name: inline def]`
   - Definitions at bottom: `[fn:1] The text...`
   - **Currently**: Not parsed — rendered as literal text

6. **Horizontal Rules** (§12.9)
   - Line of 5+ dashes → `<hr>`
   - **Currently**: Not detected

7. **Images** (§12.7, §13.9.10)
   - Image links (no description) should inline as `<img>`
   - Links to images with description become clickable image links
   - `#+CAPTION:` for figure captions
   - `#+ATTR_HTML:` for width/alt/title
   - **Currently**: Image links rendered as `<a>` text links

### Priority 2 — Important for quality/correctness

8. **`#+BEGIN_SRC` Syntax Highlighting**
   - Language-specific CSS classes for code highlighting
   - We don't need full highlight.js but should at least emit `<code class="language-X">`
   - Could include highlight.js CDN link for client-side highlighting

9. **Table of Contents Generation** (§13.3)
   - Auto-generated TOC from headings
   - Controllable depth via `#+OPTIONS: toc:2`
   - `#+OPTIONS: toc:nil` to disable
   - **Currently**: No TOC in exported HTML

10. **Export Settings / OPTIONS** (§13.2)
    - `#+OPTIONS: toc:N num:nil tags:nil todo:nil` etc.
    - Controls what gets exported (timestamps, tags, TODO keywords, drawers)
    - `#+OPTIONS: broken-links:mark` for handling broken links
    - **Currently**: All options hard-coded

11. **`CUSTOM_ID` Property for Anchors** (§4.2, §13.9.7)
    - `#[[#custom-id]]` internal link targets
    - Headlines get `id` from `CUSTOM_ID` (preferred) > `ID` > generated slug
    - **Currently**: We use `ID` or slug, but don't check `CUSTOM_ID`

12. **Internal Links** (§4.2)
    - `[[#my-custom-id]]` → link to heading with that CUSTOM_ID
    - `[[*Heading Text]]` → link to heading by name
    - `[[target]]` → link to `<<target>>` dedicated target
    - **Currently**: Only `id:` links are resolved; other internal link types ignored

13. **File Links to Other Org Files** (§13.9.8)
    - `[[file:other.org]]` → rewrite to `other.html`
    - `[[file:other.org::*heading]]` → `other.html#heading-slug`
    - `[[file:other.org::#custom-id]]` → `other.html#custom-id`
    - **Currently**: `file:` links are passed through as-is

14. **Line Breaks** (§12.1)
    - `\\` at end of line forces a line break (`<br>`)
    - **Currently**: Not handled

15. **Comment Lines** (§13.6)
    - Lines starting with `# ` should be excluded from export
    - `#+BEGIN_COMMENT` ... `#+END_COMMENT` blocks
    - Trees tagged `:noexport:` should be excluded
    - **Currently**: Comment lines may leak into HTML body

### Priority 3 — Nice to have

16. **Subscripts and Superscripts** (§12.3)
    - `a_b` → a<sub>b</sub>, `a^b` → a<sup>b</sup>
    - `a_{long}`, `a^{long}` for multi-char
    - **Currently**: Not handled

17. **Special Symbols / Entities** (§12.4)
    - `\alpha` → α, `\to` → →, `\nbsp` → &nbsp;
    - `--` → en-dash, `---` → em-dash, `...` → ellipsis
    - **Currently**: Not handled

18. **LaTeX Fragments** (§12.5, §13.9.11)
    - `$inline$`, `\(inline\)`, `$$display$$`, `\[display\]`
    - `\begin{equation}...\end{equation}`
    - Render with MathJax CDN include
    - **Currently**: Not handled

19. **Captions** (§12.8)
    - `#+CAPTION: text` before tables, images, code blocks
    - `#+CAPTION[short]: long caption`
    - **Currently**: Not handled

20. **ATTR_HTML Attributes** (§13.9.6, §13.9.8)
    - `#+ATTR_HTML: :width 300 :alt text :class myclass`
    - Applied to next element (image, table, block)
    - **Currently**: Not parsed

21. **Drawers** (§2.7)
    - Custom drawers: `:DRAWERNAME:` ... `:END:`
    - `LOGBOOK` drawer (state changes, clocking)
    - Export controlled by `d` option
    - **Currently**: Only `:PROPERTIES:` drawer recognized

22. **Clocking** (§8.4)
    - `CLOCK:` lines in LOGBOOK drawers
    - Clock tables
    - **Currently**: Not parsed

23. **Radio Targets** (§4.3)
    - `<<<My Target>>>` auto-links all occurrences
    - **Currently**: Not supported

24. **Macros** (§13.5)
    - `#+MACRO: name replacement text`
    - `{{{name(arg1,arg2)}}}` expansion
    - **Currently**: Not supported

25. **Include Files** (§13.4)
    - `#+INCLUDE: "file.org"` — includes another file
    - `#+INCLUDE: "file.org" src python` — as source block
    - **Currently**: Not supported

---

## Implementation Plan

### Phase 1: Plain Lists (highest impact)
- Parse unordered (`-`, `+`, indented `*`), ordered (`1.`, `1)`), description (`term :: desc`)
- Handle nesting via indentation
- Parse checkboxes `[ ]`, `[X]`, `[-]`
- Render as `<ul>`, `<ol>`, `<dl>` with proper nesting
- Tests: ~20 new tests

### Phase 2: Source Blocks & Literal Examples
- Parse `#+BEGIN_SRC lang` / `#+END_SRC`, `#+BEGIN_EXAMPLE` / `#+END_EXAMPLE`
- Parse fixed-width lines (`: text`)
- Render as `<pre><code class="language-X">` for src, `<pre>` for example
- Include highlight.js CDN for client-side syntax highlighting
- Tests: ~15 new tests

### Phase 3: Tables
- Parse pipe-delimited tables with header separator
- Handle column alignment hints
- Render as `<table>` with `<thead>`, `<tbody>`
- Tests: ~12 new tests

### Phase 4: Block Elements
- Parse QUOTE, VERSE, CENTER, EXPORT html blocks
- Generic `#+BEGIN_name` → `<div class="name">`
- Render appropriately
- Tests: ~10 new tests

### Phase 5: Footnotes
- Parse footnote references and definitions
- Render as numbered superscript links + footnote section at bottom
- Tests: ~8 new tests

### Phase 6: Horizontal Rules, Line Breaks, Comments, Images
- `-----` → `<hr>`
- `\\` → `<br>`
- `# ` lines and `#+BEGIN_COMMENT` excluded from export
- `:noexport:` tag excludes subtree
- Image links → `<img>` tags
- Tests: ~12 new tests

### Phase 7: CUSTOM_ID, Internal Links, File Links
- Use `CUSTOM_ID` property for heading anchors (takes precedence)
- Resolve `[[#custom-id]]`, `[[*Heading]]` internal links
- Rewrite `[[file:other.org]]` → `other.html`
- Tests: ~10 new tests

### Phase 8: Table of Contents
- Auto-generate TOC from headings
- Respect `#+OPTIONS: toc:N` depth control
- Tests: ~5 new tests

### Phase 9: Special Symbols, Entities, LaTeX
- Parse `\entity` names → Unicode/HTML entities
- Parse `--`, `---`, `...` → dashes, ellipsis
- Detect LaTeX fragments, include MathJax CDN
- Tests: ~10 new tests

### Phase 10: Captions, ATTR_HTML, Export Options
- Parse `#+CAPTION:` and `#+ATTR_HTML:` and attach to next element
- Parse `#+OPTIONS:` and respect export settings
- Tests: ~10 new tests

---

## Total Estimated New Tests: ~112
## Current Tests: 163
## Projected Final Total: ~275 tests
