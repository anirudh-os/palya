#!/usr/bin/env python3
import os
import shutil
import random
import re
import sys
from datetime import date, timedelta

# Configuration
NUM_POSTS = int(sys.argv[1]) if len(sys.argv) > 1 else 7500
BENCH_DIR = os.path.abspath("ssg_benchmark")

TITLES = [
    "Understanding Rust Ownership", "A Deep Dive into Async Await",
    "Building CLI Tools", "WebAssembly from Scratch", "Error Handling Patterns",
    "The Art of Benchmarking", "Memory Safety in Systems Code", "Zero Cost Abstractions",
    "Concurrency Without Fear", "Lifetimes Explained", "Trait Objects vs Generics",
    "Writing Fast Parsers", "Embedded Rust Primer", "From C to Rust",
    "Fearless Refactoring", "Iterators in Depth", "Smart Pointers Unpacked",
    "Testing Strategies", "Building a Static Site Generator", "Macros Demystified"
]

TAGS = [
    "rust", "systems", "performance", "webassembly", "cli", "async", "concurrency",
    "memory", "tooling", "parsing", "embedded", "testing", "macros", "generics",
    "lifetimes", "iterators", "traits", "benchmarking", "security"
]

COLLECTIONS = ["blog", "projects", "notes", "tutorials"]
# Palya maps 'projects' to a different context key (project vs post),
# so restrict benchmark posts to collections that all use the 'post' context.
PALYA_COLLECTIONS = ["blog", "notes", "tutorials"]


def slugify(text):
    text = text.lower()
    return re.sub(r'[^a-z0-9\-]', '', text.replace(' ', '-'))


def rand_date():
    start_date = date(2020, 1, 1)
    end_date = date(2025, 12, 31)
    days_between_dates = (end_date - start_date).days
    return start_date + timedelta(days=random.randrange(days_between_dates))


def gen_body(title):
    return f"""
## Introduction

This post covers **{title}** in detail. Whether you are just starting out or
looking to deepen your understanding, the concepts here should prove useful.

```rust
fn main() {{
    let message = greet("world");
    println!("{{}}", message);
}}

fn greet(name: &str) -> String {{
    format!("Hello, {{}}!", name)
}}
```

## Core Concepts

There are three fundamental ideas to keep in mind:

1. **Correctness** – the program does what it is supposed to do.
2. **Performance** – it does so without wasting resources.
3. **Maintainability** – future contributors (including yourself) can understand it.

## A Deeper Look

Consider the following pattern, which avoids unnecessary allocation:

```rust
pub fn process<T: AsRef<str>>(input: T) -> usize {{
    input.as_ref().chars().filter(|c| c.is_alphabetic()).count()
}}
```

This leverages a generic bound so callers can pass either `&str` or `String`
without any extra ceremony.

## Conclusion

Mastering these ideas takes time, but the payoff in terms of confidence and code
quality is well worth the investment. Keep experimenting, reading the docs, and
above all, compiling your code often.
"""


def scaffold_palya():
    root = os.path.join(BENCH_DIR, "palya")
    os.makedirs(os.path.join(root, "templates"), exist_ok=True)
    os.makedirs(os.path.join(root, "static"), exist_ok=True)

    with open(os.path.join(root, "palya.toml"), "w") as f:
        f.write('author = "Benchmark Bot"\ntitle  = "Palya Bench Site"\ndescription = "Benchmarking Palya"\nbase_url = ""\n')

    templates = {
        "base.j2": '<!DOCTYPE html>\n<html lang="en">\n<head><meta charset="UTF-8"><title>{% block title %}{{ site.title }}{% endblock %}</title></head>\n<body>{% block content %}{% endblock %}</body>\n</html>',
        "post.j2": '{% extends "base.j2" %}\n{% block title %}{{ post.frontmatter.title }} · {{ site.title }}{% endblock %}\n{% block content %}\n<article>\n  <h1>{{ post.frontmatter.title }}</h1>\n  <p>{{ post.frontmatter.date }}</p>\n  {% if post.tags %}\n  <ul>{% for tag in post.tags %}<li><a href="/tags/{{ tag }}.html">{{ tag }}</a></li>{% endfor %}</ul>\n  {% endif %}\n  {{ post.content | safe }}\n</article>\n{% endblock %}',
        "page.j2": '{% extends "base.j2" %}\n{% block title %}{{ page.frontmatter.title }} · {{ site.title }}{% endblock %}\n{% block content %}\n<article>\n  <h1>{{ page.frontmatter.title }}</h1>\n  {{ page.content | safe }}\n</article>\n{% endblock %}',
        "project.j2": '{% extends "base.j2" %}\n{% block title %}{{ project.frontmatter.title }} · {{ site.title }}{% endblock %}\n{% block content %}\n<article>\n  <h1>{{ project.frontmatter.title }}</h1>\n  {{ project.content | safe }}\n</article>\n{% endblock %}',
        "index.j2": '{% extends "base.j2" %}\n{% block content %}\n<ul>\n{% for post in posts %}\n  <li><a href="{{ post.url }}">{{ post.frontmatter.title }}</a></li>\n{% endfor %}\n</ul>\n{% endblock %}',
        "tag.j2": '{% extends "base.j2" %}\n{% block content %}\n<h1>Tag: {{ tag }}</h1>\n<ul>{% for post in posts %}<li><a href="{{ post.url }}">{{ post.frontmatter.title }}</a></li>{% endfor %}</ul>\n{% endblock %}'
    }

    for name, content in templates.items():
        with open(os.path.join(root, "templates", name), "w") as f:
            f.write(content)


def scaffold_hugo():
    root = os.path.join(BENCH_DIR, "hugo")
    os.makedirs(os.path.join(root, "layouts", "_default"), exist_ok=True)
    os.makedirs(os.path.join(root, "static"), exist_ok=True)

    with open(os.path.join(root, "hugo.toml"), "w") as f:
        f.write('baseURL  = "http://localhost/"\ntitle    = "Hugo Bench Site"\n')

    layouts = {
        "index.html": '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>{{ .Site.Title }}</title></head>\n<body>\n{{ range .Pages }}<a href="{{ .Permalink }}">{{ .Title }}</a><br>{{ end }}\n</body></html>',
        "_default/single.html": '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>{{ .Title }}</title></head>\n<body><article><h1>{{ .Title }}</h1><p>{{ .Date.Format "2006-01-02" }}</p>{{ .Content }}</article></body></html>',
        "_default/list.html": '<!DOCTYPE html><html><head><meta charset="UTF-8"><title>{{ .Title }}</title></head>\n<body><ul>{{ range .Pages }}<li><a href="{{ .Permalink }}">{{ .Title }}</a></li>{{ end }}</ul></body></html>'
    }

    for name, content in layouts.items():
        with open(os.path.join(root, "layouts", name), "w") as f:
            f.write(content)

    for section in COLLECTIONS:
        sec_dir = os.path.join(root, "content", section)
        os.makedirs(sec_dir, exist_ok=True)
        with open(os.path.join(sec_dir, "_index.md"), "w") as f:
            f.write(f'---\ntitle: "{section.capitalize()}"\n---\n')


def scaffold_zola():
    root = os.path.join(BENCH_DIR, "zola")
    os.makedirs(os.path.join(root, "templates"), exist_ok=True)
    os.makedirs(os.path.join(root, "static"), exist_ok=True)
    os.makedirs(os.path.join(root, "sass"), exist_ok=True)

    with open(os.path.join(root, "config.toml"), "w") as f:
        f.write('base_url     = "http://localhost"\ntitle        = "Zola Bench Site"\ncompile_sass = false\nbuild_search_index = false\n\n[[taxonomies]]\nname = "tags"\n')

    templates = {
        "base.html": '<!DOCTYPE html>\n<html lang="en">\n<head><meta charset="UTF-8"><title>{% block title %}{{ config.title }}{% endblock %}</title></head>\n<body>{% block content %}{% endblock %}</body>\n</html>',
        "index.html": '{% extends "base.html" %}\n{% block content %}\n<ul>\n{% for page in section.pages %}\n  <li><a href="{{ page.permalink }}">{{ page.title }}</a></li>\n{% endfor %}\n</ul>\n{% endblock %}',
        "page.html": '{% extends "base.html" %}\n{% block title %}{{ page.title }}{% endblock %}\n{% block content %}\n<article>\n  <h1>{{ page.title }}</h1>\n  <p>{{ page.date }}</p>\n  {{ page.content | safe }}\n</article>\n{% endblock %}',
        "section.html": '{% extends "base.html" %}\n{% block title %}{{ section.title }}{% endblock %}\n{% block content %}\n<ul>\n{% for page in section.pages %}\n  <li><a href="{{ page.permalink }}">{{ page.title }}</a></li>\n{% endfor %}\n</ul>\n{% endblock %}',
        "taxonomy_list.html": '{% extends "base.html" %}\n{% block content %}\n<ul>{% for term in terms %}<li><a href="{{ term.permalink }}">{{ term.name }}</a></li>{% endfor %}</ul>\n{% endblock %}',
        "taxonomy_single.html": '{% extends "base.html" %}\n{% block content %}\n<h1>{{ term.name }}</h1>\n<ul>{% for page in term.pages %}<li><a href="{{ page.permalink }}">{{ page.title }}</a></li>{% endfor %}</ul>\n{% endblock %}'
    }

    for name, content in templates.items():
        with open(os.path.join(root, "templates", name), "w") as f:
            f.write(content)

    for section in COLLECTIONS:
        sec_dir = os.path.join(root, "content", section)
        os.makedirs(sec_dir, exist_ok=True)
        with open(os.path.join(sec_dir, "_index.md"), "w") as f:
            f.write(f'+++\ntitle = "{section.capitalize()}"\nsort_by = "date"\n+++\n')

    with open(os.path.join(root, "content", "_index.md"), "w") as f:
        f.write('+++\ntitle = "Home"\n+++\n')


def generate_posts():
    for i in range(1, NUM_POSTS + 1):
        raw_title = f"{random.choice(TITLES)} {i}"
        slug = slugify(raw_title)
        d = rand_date().isoformat()
        tag1, tag2 = random.sample(TAGS, 2)
        collection = random.choice(COLLECTIONS)
        palya_collection = random.choice(PALYA_COLLECTIONS)
        body = gen_body(raw_title)

        # Palya (only blog/notes/tutorials so every post gets the 'post' context key)
        p_dir = os.path.join(BENCH_DIR, "palya", "content", palya_collection)
        os.makedirs(p_dir, exist_ok=True)
        with open(os.path.join(p_dir, f"{slug}.md"), "w") as f:
            f.write(f'---\ntitle: {raw_title}\ndate: {d}\nslug: {slug}\ntemplate: post.j2\ndraft: false\ntags:\n  - {tag1}\n  - {tag2}\n---\n{body}')

        # Hugo
        h_dir = os.path.join(BENCH_DIR, "hugo", "content", collection)
        os.makedirs(h_dir, exist_ok=True)
        with open(os.path.join(h_dir, f"{slug}.md"), "w") as f:
            f.write(f'---\ntitle: "{raw_title}"\ndate: {d}T00:00:00Z\ndraft: false\ntags:\n  - {tag1}\n  - {tag2}\n---\n{body}')

        # Zola
        z_dir = os.path.join(BENCH_DIR, "zola", "content", collection)
        os.makedirs(z_dir, exist_ok=True)
        with open(os.path.join(z_dir, f"{slug}.md"), "w") as f:
            f.write(f'+++\ntitle = "{raw_title}"\ndate = {d}\ndraft = false\n\n[taxonomies]\ntags = ["{tag1}", "{tag2}"]\n+++\n{body}')

        if i % 1000 == 0:
            print(f"[INFO] Generated {i}/{NUM_POSTS}")


if __name__ == "__main__":
    print("[INFO] Wiping previous benchmark dir...")
    if os.path.exists(BENCH_DIR):
        shutil.rmtree(BENCH_DIR)

    print("[INFO] Scaffolding sites...")
    scaffold_palya()
    scaffold_hugo()
    scaffold_zola()

    print(f"[INFO] Generating {NUM_POSTS} posts...")
    generate_posts()
    print("[INFO] Done!")