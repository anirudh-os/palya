# Palya

Palya is a minimal, fast static site generator written in Rust.  
It focuses on simplicity, performance, and flexibility while remaining easy to configure and extend.

## Features

- **Fast builds:** Parallel processing using Rayon to utilize all CPU cores.
- **Jinja-style templates:** Powered by MiniJinja with full support for `.j2` templates.
- **Single static binary:** No runtime dependencies — download and run.
- **Simple configuration:** Optional `palya.toml` file for site-wide settings.

## Installation

Download the latest prebuilt binary for your platform from the
[GitHub Releases page](https://github.com/anirudh-os/palya/releases).

Then make it executable (Linux/macOS):

```bash
chmod +x palya
```

## Usage

```bash
# Build a site
./palya \
  --input path_to_content_directory \
  --output path_to_public_directory \
  --templates path_to_templates_directory

# Show all options
./palya --help
```

## Project Structure

A typical Palya site looks like this:

```bash
my_site/
├── content/        # Markdown content
├── static/         # Static assets (CSS, images. etc.)
├── templates/      # Jinja (.j2) templates
├── palya.toml      # Optional configuration file
```

## Configuration (palya.toml)

The configuration file is optional.
If present, it must be located in the site's root directory.

```toml
title = "My Awesome Blog"
description = "My description"
base_url = "https://mysite.com"
```

These values are available inside templates.

## Templates

Palya uses Jinja-style templates (`.j2`) via MiniJinja.

The example site includes:

- `index.j2` --- homepage template

- `post.j2` --- individual post template

You are free to design and structure templates however you like.

### Template Context Keys

The variable name available in a template depends on which *content collection* the file belongs to (i.e. which subdirectory under `content/` it lives in):

| Collection (`content/<dir>/`) | Context variable |
|-------------------------------|-----------------|
| `blog/`, `notes/`, `tutorials/`, etc. | `post` |
| `projects/` | `project` |
| `pages/` | `page` |

So in a `post.j2` template you access `{{ post.frontmatter.title }}`, while in a `project.j2` template you must use `{{ project.frontmatter.title }}`. Using the wrong variable name will result in an *undefined value* error at build time.

## Example

An example site is provided in the `example_site/` directory to help you get started quickly.

## License

Palya is licensed under the MIT License.  
See the [LICENSE](LICENSE) file for details.