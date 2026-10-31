<p align="center">
  <img src=".github/brand/awesomeify-icon.svg" alt="" width="96" height="96">
</p>

<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/brand/awesomeify-wordmark-dark.svg">
    <img alt="Awesomeify" src=".github/brand/awesomeify-wordmark-light.svg" height="56">
  </picture>
</h1>

<p align="center">
  <em>Index GitHub repositories and generate static awesome-list sites.</em>
</p>

Awesomeify reads repositories from GitHub search queries and from existing awesome lists, stores them
in SQLite with an activity score, and generates a static, searchable site (with Pagefind) where the lists
are grouped by language and topic.

## Usage

```bash
awesomeify index --query "stars:>100 language:rust"   # index from a GitHub search
awesomeify index --seed seeds/awesome-lists.toml      # or from existing awesome lists
awesomeify generate --output ./dist --active-only     # write the static site
awesomeify serve --port 3000                          # preview it locally
awesomeify stats                                      # what is in the database
```

With [mise](https://mise.jdx.dev/), `mise run full` indexes, generates and builds the search index in one go.

## License

MIT
