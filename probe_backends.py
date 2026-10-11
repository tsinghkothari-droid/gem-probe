#!/usr/bin/env python3
"""Probe whether a portal exposes tender rows on any public page.

One challenge portal at a time. No captcha, OTP, or login is submitted.
We only follow the site's own links and try a few known page names.
"""

import json
import re
import ssl
import urllib.request
from html import unescape
from urllib.parse import urljoin
from concurrent.futures import ThreadPoolExecutor, as_completed

UA = {"User-Agent": "gem-probe/0.1 (public tender-list probe)"}
CTX = ssl._create_unverified_context()
TENDER_ID = re.compile(r"20\d{2}_[A-Z0-9]{2,}")
TOTAL = re.compile(r"Total\s+(Tenders?|Bid\(s\)|Archived)\s*[:\s]\s*([\d,]{2,12})", re.I)

GEPNIC_PAGES = [
    ("active", "app?page=FrontEndLatestActiveTenders&service=page"),
    ("orgwise", "app?page=FrontEndLatestActiveTendersOrgwise&service=page"),
    ("by-date", "app?page=FrontEndListTendersbyDate&service=page"),
    ("closing", "app?page=FrontEndTendersByDate&service=page"),
]


def fetch(url):
    req = urllib.request.Request(url, headers=UA)
    try:
        with urllib.request.urlopen(req, timeout=18) as response:
            return response.status, response.geturl(), response.read(400_000).decode("utf-8", "replace"), ""
    except Exception as first:
        if "CERTIFICATE_VERIFY_FAILED" not in str(first) and "UNSAFE_LEGACY" not in str(first):
            return 0, url, "", str(first)[:120]
    try:
        with urllib.request.urlopen(req, timeout=18, context=CTX) as response:
            return response.status, response.geturl(), response.read(400_000).decode("utf-8", "replace"), "unverified-tls"
    except Exception as second:
        return 0, url, "", str(second)[:120]


def challenge(html):
    low = html.lower()
    if "authenticate yourself" in low or ("otp" in low and "mobile number" in low):
        return "otp"
    if "provide captcha" in low or "what code is in the image" in low:
        return "captcha"
    if "captcha" in low:
        return "captcha"
    return "none"


def looks_like_rows(html):
    ids = set(TENDER_ID.findall(html))
    gem_ids = set(re.findall(r"GEM/20\d\d/[A-Z]/", html))
    totals = TOTAL.findall(unescape(re.sub(r"<[^>]+>", " ", html)))
    return ids, gem_ids, totals


def try_urls(name_url_pairs):
    out = []
    for name, url in name_url_pairs:
        status, final, html, err = fetch(url)
        if not html:
            out.append({"name": name, "url": url, "status": status, "error": err, "challenge": "error", "ids": 0, "total": None})
            continue
        ids, gem_ids, totals = looks_like_rows(html)
        out.append({
            "name": name,
            "url": url,
            "status": status,
            "error": err,
            "challenge": challenge(html),
            "ids": len(ids) + len(gem_ids),
            "total": totals[0][1] if totals else None,
            "titles": bool(ids or gem_ids),
        })
        if ids or gem_ids:
            break
    return out


def site_links(html, base, limit=4):
    hrefs = re.findall(r'href=["\']([^"\']+)["\']', html, re.I)
    picked = []
    for href in hrefs:
        low = href.lower()
        if any(word in low for word in ("tender", "active", "archive", "search", "bid")):
            if href.startswith(("javascript", "#", "mailto")):
                continue
            full = urljoin(base, href)
            if full not in picked and full.startswith("http"):
                picked.append(full)
        if len(picked) >= limit:
            break
    return picked


def probe(host):
    home_url = host.rstrip("/") + "/"
    status, final, html, err = fetch(home_url)
    result = {"host": host, "home": {"status": status, "error": err, "challenge": challenge(html) if html else "error"}, "pages": []}
    if not html:
        return result

    attempts = []
    for marker, suffix in GEPNIC_PAGES:
        if "/app" in final or "/app" in html[:20_000].lower():
            base = final.split("?")[0]
            if not base.rstrip("/").endswith("app"):
                match = re.search(r'https?://[^"\'\s]+/(?:nicgep|eprocure|epublish)/app', html)
                base = match.group(0) if match else None
            if base:
                attempts.append((marker, f"{base}?{suffix.split('?', 1)[1]}"))
        else:
            attempts.append((marker, urljoin(home_url, f"nicgep/{suffix}")))
    result["pages"].extend(try_urls(attempts[:4]))

    if not any(page.get("titles") for page in result["pages"]):
        links = site_links(html, final or home_url)
        result["pages"].extend(try_urls([(f"link:{i}", link) for i, link in enumerate(links)]))
    return result


HOSTS = [line for line in open("probe-results.json")]
rows = json.load(open("probe-results.json"))
CHALLENGE_HOSTS = [row["url"] for row in rows if row["group"] == "challenge-required"]
EMPTY_HOSTS = [row["url"] for row in rows if row["group"] in {"empty-shell"}]


def main():
    all_rows = []
    with ThreadPoolExecutor(max_workers=5) as pool:
        futures = {pool.submit(probe, url): url for url in CHALLENGE_HOSTS + EMPTY_HOSTS}
        for future in as_completed(futures):
            result = future.result()
            all_rows.append(result)
            opened = [page for page in result["pages"] if page.get("titles")]
            if opened:
                page = opened[0]
                print(f"OPEN  {page['ids']:3} ids total={page['total']}  {page['url'][:100]}")
            else:
                blocked = [page for page in result["pages"] if page.get("challenge") in {"captcha", "otp"}]
                why = blocked[0]["challenge"] if blocked else "no rows"
                print(f"GATE  {why:8}                    {result['host'][:100]}")
    all_rows.sort(key=lambda item: item["host"])
    with open("backend-probe-results.json", "w", encoding="utf-8") as handle:
        json.dump(all_rows, handle, indent=2)
        handle.write("\n")
    opened = sum(1 for item in all_rows if any(page.get("titles") for page in item["pages"]))
    print(f"\n{opened} of {len(all_rows)} exposed rows on a public page")


if __name__ == "__main__":
    main()