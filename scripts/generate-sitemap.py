#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright 2026 Tom F.
#
# Generates sitemap.xml from book/src/SUMMARY.md.
# Run from repo root: python3 scripts/generate-sitemap.py

import re
from pathlib import Path

SITE_URL = "https://quack-rs.com"
SUMMARY = Path("book/src/SUMMARY.md")
OUTPUT = Path("book/src/sitemap.xml")

# Priority tiers based on path depth and importance
PRIORITY_OVERRIDES = {
    "/": "1.0",
    "/getting-started/quick-start.html": "0.9",
}
CHANGEFREQ_OVERRIDES = {
    "/": "weekly",
    "/reference/changelog.html": "weekly",
}


def md_to_html(md_path: str) -> str:
    """Convert a .md path to its mdBook HTML output path."""
    if md_path == "introduction.md":
        return "/"
    return "/" + md_path.replace(".md", ".html")


def priority_for(html_path: str) -> str:
    if html_path in PRIORITY_OVERRIDES:
        return PRIORITY_OVERRIDES[html_path]
    depth = html_path.strip("/").count("/")
    if depth == 0:
        return "0.7"
    if depth == 1:
        return "0.7"
    return "0.6"


def changefreq_for(html_path: str) -> str:
    return CHANGEFREQ_OVERRIDES.get(html_path, "monthly")


def parse_summary(text: str) -> list[str]:
    """Extract .md paths from SUMMARY.md link entries."""
    paths = []
    for match in re.finditer(r"\[.*?\]\((.*?\.md)\)", text):
        paths.append(match.group(1))
    return paths


def main() -> None:
    summary_text = SUMMARY.read_text()
    md_paths = parse_summary(summary_text)

    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ]

    for md_path in md_paths:
        html_path = md_to_html(md_path)
        loc = SITE_URL + html_path
        freq = changefreq_for(html_path)
        prio = priority_for(html_path)
        lines.append(
            f"  <url>"
            f"<loc>{loc}</loc>"
            f"<changefreq>{freq}</changefreq>"
            f"<priority>{prio}</priority>"
            f"</url>"
        )

    lines.append("</urlset>")
    lines.append("")

    OUTPUT.write_text("\n".join(lines))
    print(f"Generated {OUTPUT} with {len(md_paths)} URLs")


if __name__ == "__main__":
    main()
