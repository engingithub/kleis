#!/usr/bin/env python3
"""
Generate sitemap.xml for kleis.io from the manual source (.md) files.

This script reads the mdBook source files and generates a sitemap
with the expected HTML URLs. Run this locally before committing.

Usage:
    python3 scripts/generate_sitemap.py

Output:
    sitemap.xml in the repository root
"""

import re
import subprocess
from datetime import datetime
from pathlib import Path
from typing import List, Tuple

# Configuration
REPO_ROOT = Path(__file__).parent.parent
BASE_URL = "https://kleis.io"
MANUAL_SRC = REPO_ROOT / "docs" / "manual" / "src"
SUMMARY_FILE = MANUAL_SRC / "SUMMARY.md"
OUTPUT_FILE = REPO_ROOT / "sitemap.xml"

# Priority settings (checked in order; first match wins)
PRIORITY_MAP = [
    ("/docs/manual/book/", 0.9),
    ("chapters/", 0.8),
    ("appendix/", 0.6),
]

# Pages to exclude (duplicates of other entries)
EXCLUDE_URLS = {
    "/docs/manual/book/introduction",  # identical to index
}

# Additional resources not discovered from SUMMARY.md (PDFs, data files, etc.)
# Format: (source_description, url_path, priority)
EXTRA_ENTRIES = [
    ("docs/papers/spectral_comb_proof.pdf",
     "/docs/papers/spectral_comb_proof.pdf", 1.0),
    ("docs/papers/transfer_function_paper.pdf",
     "/docs/papers/transfer_function_paper.pdf", 1.0),
    ("docs/papers/pot_flat_rotation_curves.pdf",
     "/docs/papers/pot_flat_rotation_curves.pdf", 1.0),
    ("docs/papers/pot_electrodynamics_paper.pdf",
     "/docs/papers/pot_electrodynamics_paper.pdf", 1.0),
    ("docs/papers/pot_yang_mills_paper.pdf",
     "/docs/papers/pot_yang_mills_paper.pdf", 1.0),
]


def get_priority(path: str) -> float:
    """Determine priority based on path patterns."""
    if path == "/":
        return 1.0
    for _, url, pri in EXTRA_ENTRIES:
        if path == url:
            return pri
    for pattern, priority in PRIORITY_MAP:
        if pattern in path:
            return priority
    return 0.5


def parse_summary() -> List[Tuple[str, str]]:
    """
    Parse SUMMARY.md to extract all linked .md files.
    Returns list of (md_path, expected_html_url).
    """
    pages = []
    
    if not SUMMARY_FILE.exists():
        print(f"Warning: {SUMMARY_FILE} not found")
        return pages
    
    content = SUMMARY_FILE.read_text()
    
    # Match markdown links: [Title](path/to/file.md)
    link_pattern = re.compile(r'\[([^\]]+)\]\(([^)]+\.md)\)')
    
    for match in link_pattern.finditer(content):
        title = match.group(1)
        md_path = match.group(2)
        
        # Convert .md path to clean URL (no .html extension).
        # Cloudflare Pages 308-redirects .html → clean URL, so sitemap
        # must use the final URL to avoid Google "Page with redirect" errors.
        # ./chapters/01-starting-out.md → /docs/manual/book/chapters/01-starting-out
        clean_path = md_path.replace('.md', '')
        if clean_path.startswith('./'):
            clean_path = clean_path[2:]
        url = f"/docs/manual/book/{clean_path}"
        
        pages.append((md_path, url))
    
    return pages


def get_static_pages() -> List[Tuple[str, str]]:
    """Return list of static pages (landing page, etc.)."""
    pages = []
    
    # Landing page
    if (REPO_ROOT / "index.html").exists():
        pages.append(("index.html", "/"))
    
    # Manual index (clean URL = directory root)
    pages.append(("docs/manual/src/SUMMARY.md", "/docs/manual/book/"))
    
    return pages


def git_last_modified(filepath: str) -> str:
    """Get the last git commit date for a file, or today if untracked."""
    resolved = MANUAL_SRC / filepath if not Path(filepath).is_absolute() else Path(filepath)
    if not resolved.exists():
        resolved = REPO_ROOT / filepath
    try:
        result = subprocess.run(
            ["git", "log", "-1", "--format=%aI", "--", str(resolved)],
            capture_output=True, text=True, cwd=REPO_ROOT, timeout=5,
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()[:10]
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass
    return datetime.now().strftime("%Y-%m-%d")


def generate_sitemap(pages: List[Tuple[str, str]]) -> str:
    """Generate sitemap XML content."""
    xml_parts = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ]
    
    seen_urls = set()
    
    for source, url_path in pages:
        if url_path in seen_urls or url_path in EXCLUDE_URLS:
            continue
        seen_urls.add(url_path)
        
        priority = get_priority(url_path)
        lastmod = git_last_modified(source)
        
        xml_parts.append(f"""  <url>
    <loc>{BASE_URL}{url_path}</loc>
    <lastmod>{lastmod}</lastmod>
    <priority>{priority}</priority>
  </url>""")
    
    xml_parts.append("</urlset>")
    return "\n".join(xml_parts)


def main():
    print(f"🔍 Reading manual structure from {SUMMARY_FILE}...")
    
    # Get static pages
    static_pages = get_static_pages()
    print(f"   Found {len(static_pages)} static pages")
    
    # Parse SUMMARY.md for manual pages
    manual_pages = parse_summary()
    print(f"   Found {len(manual_pages)} manual pages in SUMMARY.md")
    
    # Extra entries (PDFs, research papers, etc.)
    extra_pages = [(src, url) for src, url, _ in EXTRA_ENTRIES]
    print(f"   Found {len(extra_pages)} extra entries (papers, etc.)")

    all_pages = static_pages + manual_pages + extra_pages
    
    print(f"\n📝 Generating sitemap...")
    sitemap_content = generate_sitemap(all_pages)
    
    print(f"💾 Writing to {OUTPUT_FILE}...")
    OUTPUT_FILE.write_text(sitemap_content)
    
    url_count = sitemap_content.count('<url>')
    print(f"\n✅ Sitemap generated with {url_count} URLs")
    print(f"   Output: {OUTPUT_FILE}")
    
    # Print summary
    print("\n📊 URL breakdown:")
    categories = {"Landing": 0, "Chapters": 0, "Appendix": 0, "Other": 0}
    for _, url in all_pages:
        if url == "/":
            categories["Landing"] += 1
        elif "/chapters/" in url:
            categories["Chapters"] += 1
        elif "/appendix/" in url:
            categories["Appendix"] += 1
        else:
            categories["Other"] += 1
    
    for cat, count in sorted(categories.items()):
        if count > 0:
            print(f"   - {cat}: {count}")
    
    print("\n💡 Next steps:")
    print("   1. Review sitemap.xml")
    print("   2. git add sitemap.xml")
    print("   3. Commit with your other changes")


if __name__ == "__main__":
    main()
