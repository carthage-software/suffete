# Contributing

The book lives in `book/` of the suffete repository. It's an mdbook with mathjax and mermaid enabled. Contributions are welcome — chapter improvements, fixes for incorrect descriptions, new cookbook recipes, missing diagrams.

## Building locally

Install mdbook and the mermaid preprocessor:

```sh
cargo install mdbook
cargo install mdbook-mermaid
```

Then, from the repo root:

```sh
cd book
mdbook-mermaid install .   # one-time, copies mermaid assets into theme/
mdbook serve --open        # builds and watches; opens in browser
```

The book rebuilds on file changes; the browser auto-reloads.

## Style

Match the existing chapters:

- **Dense but readable.** No filler. Short paragraphs. Real examples in every chapter.
- **Math via mathjax.** `$\tau \mathrel{<:} \sigma$`, `$\tau \sqcup \sigma$`, etc.
- **Diagrams via mermaid** when they clarify (not as decoration).
- **Code samples are `rust,ignore`** ; the book is not a doctest harness.
- **PHP samples are `php`** ; PHP is not compiled by mdbook either, but the syntax highlighting is helpful.
- **"Element" not "atom"** in code-related text. The atom note is in the [introduction](./introduction.md) and [glossary](./foundations/glossary.md).
- **No emojis** anywhere ; book or commit messages.
- **No em-dashes** anywhere.
- **Cross-link** to other chapters when a concept comes up that has its own treatment.
- **Every chapter ends with a "see also" footer.**

## Naming chapter files

Chapter files live in `book/src/<part>/<name>.md`. The file name is `kebab-case`. The `SUMMARY.md` references each chapter by its full path.

When you add a chapter, register it in `SUMMARY.md` ; otherwise it doesn't render.

## Verifying

Before submitting:

```sh
cd book
mdbook build       # checks links, mermaid, mathjax
mdbook test        # smoke-tests code samples (rust,ignore is skipped, but the parser runs)
```

Open `book/book/index.html` in your browser and click through every page to check that math and diagrams render. CI does the same on every PR.

## Where to file issues

Issues that affect the book go to the [suffete repository](https://github.com/carthage-software/suffete) with a `book` label. PRs touching `book/` trigger the book CI workflow.
