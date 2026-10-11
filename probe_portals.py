#!/usr/bin/env python3
"""Public GET probe. Does not submit a captcha, OTP, or login."""

import json
import re
import ssl
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from html import unescape

UA = {"User-Agent": "gem-probe/0.1 (public tender-list probe)"}
CTX = ssl._create_unverified_context()
TIMEOUT = 20

URLS = [
    "https://eprocure.gov.in/eprocure/app",
    "https://eprocure.gov.in/epublish/app",
    "https://etenders.gov.in/",
    "https://gem.gov.in/",
    "https://bidplus.gem.gov.in/all-bids",
    "https://gepnic.gov.in/",
    "https://tender.apeprocurement.gov.in/",
    "https://arunachaltenders.gov.in/",
    "https://assamtenders.gov.in/",
    "https://eproc2.bihar.gov.in/EPSV2Web/",
    "https://cgeproc.cgstate.gov.in/",
    "https://eprocure.goa.gov.in/",
    "https://www.nprocure.com/",
    "https://etenders.hry.nic.in/",
    "https://hptenders.gov.in/",
    "https://jharkhandtenders.gov.in/",
    "https://kppp.karnataka.gov.in/",
    "https://etenders.kerala.gov.in/",
    "https://mptenders.gov.in/",
    "https://mahatenders.gov.in/nicgep/app",
    "https://manipurtenders.gov.in/",
    "https://meghalayatenders.gov.in/",
    "https://mizoramtenders.gov.in/",
    "https://nagalandtenders.gov.in/",
    "https://tendersodisha.gov.in/",
    "https://eproc.punjab.gov.in/",
    "https://shpp.rajasthan.gov.in/",
    "https://eproc.rajasthan.gov.in/",
    "https://sikkimtender.gov.in/",
    "https://tntenders.gov.in/",
    "https://tender.telangana.gov.in/",
    "https://tripuratenders.gov.in/",
    "https://etender.up.nic.in/",
    "https://uktenders.gov.in/",
    "https://tenders.wb.gov.in/",
    "https://wbtenders.gov.in/",
    "https://eprocure.andamannicobar.gov.in/",
    "https://etenders.chd.nic.in/",
    "https://dnhtenders.gov.in/",
    "https://ddtenders.gov.in/",
    "https://ddd.gov.in/notice-category/tenders/",
    "https://govtprocurement.delhi.gov.in/",
    "https://jktenders.gov.in/",
    "https://tenders.ladakh.gov.in/",
    "https://tendersutl.gov.in/",
    "https://pudutenders.gov.in/",
    "https://defproc.gov.in/",
    "https://www.ireps.gov.in/",
    "https://nbccindia.in/",
    "https://www.pmgsytenders.gov.in/",
    "https://coalindiatenders.nic.in/",
    "https://eprocurentpc.nic.in/",
    "https://eprocurebhel.co.in/",
    "https://iocletenders.nic.in/",
    "https://eprocurebel.co.in/",
    "https://eprocurehsl.nic.in/",
    "https://eprocuremdl.nic.in/",
    "https://eprocuregsl.nic.in/",
    "https://eprocure.gov.in/cppp/latestactivetendersnew/cpppdata",
    "https://eprocure.gov.in/cppp/latestactivetendersnew/mmpdata",
    "https://eprocure.gov.in/cppp/highvaluetenders/cpppdata",
    "https://eprocure.gov.in/cppp/latestactivetendersnew/gemdata",
    "https://eprocure.gov.in/cppp/tendersclosingbydays/bytoday",
    "https://eprocure.gov.in/cppp/resultoftendersnew/cpppdata",
]

KEEP = [
    "cement plaster",
    "plastering",
    "internal plaster",
    "external plaster",
    "ceiling plaster",
    "wall putty",
    "brick masonry",
    "brick work",
]
STATES = [
    "Gujarat",
    "Karnataka",
    "Maharashtra",
    "Uttar Pradesh",
    "West Bengal",
    "Rajasthan",
    "Bihar",
    "Andhra Pradesh",
    "Telangana",
    "Tamil Nadu",
]


def fetch(url):
    req = urllib.request.Request(url, headers=UA)
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as response:
            raw = response.read(500_000)
            return response.status, response.geturl(), raw.decode("utf-8", "replace"), ""
    except Exception as first:
        if "CERTIFICATE_VERIFY_FAILED" not in str(first):
            return 0, url, "", str(first)[:180]
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT, context=CTX) as response:
            raw = response.read(500_000)
            return response.status, response.geturl(), raw.decode("utf-8", "replace"), "unverified-tls"
    except Exception as second:
        return 0, url, "", str(second)[:180]


def plain(html):
    text = re.sub(r"<script[\s\S]*?</script>", " ", html, flags=re.I)
    text = re.sub(r"<style[\s\S]*?</style>", " ", text, flags=re.I)
    text = re.sub(r"<[^>]+>", " ", text)
    return re.sub(r"\s+", " ", unescape(text))


def first_int(text, patterns):
    for pattern in patterns:
        match = re.search(pattern, text, re.I)
        if match:
            return int(match.group(1).replace(",", ""))
    return None


def family_of(html, final):
    low = html.lower()
    if "bidplus.gem.gov.in" in final or "csrf_bd_gem_nk" in html:
        return "gem-bidplus"
    if "nicgep" in low or "frontendadvancedsearch" in low or "frontendlatestactivetenders" in low:
        return "gepnic"
    if "eprocurement system" in low and "captcha" in low:
        return "gepnic"
    if "app-root" in low or "runtime.js" in low:
        return "angular"
    if "wp-content" in low or "wordpress" in low:
        return "wordpress"
    if "epsV2" in html or "EPSV2" in html:
        return "bihar-eps"
    if "ireps" in low:
        return "ireps"
    if "gem.gov.in" in final:
        return "gem-home"
    return "other"


def titles_visible(html, text):
    if re.search(r"20\d\d_[A-Z0-9]{2,}", html):
        return True
    if re.search(r"GEM/20\d\d/[A-Z]/", html):
        return True
    if "Total Tenders" in text and re.search(r"\d{2}-[A-Za-z]{3}-20\d\d", text):
        return True
    if "Total Bid(s)" in text and re.search(r"GEM/20\d\d", text):
        return True
    return False


def challenge_of(html, text):
    low = html.lower()
    if "authenticate yourself" in low or ("otp" in low and "mobile number" in low):
        return "otp"
    if "provide captcha" in low or "what code is in the image" in text.lower():
        return "captcha"
    if "captcha" in low and not titles_visible(html, text):
        return "captcha"
    if re.search(r"type=[\"']password[\"']", low) and not titles_visible(html, text):
        return "login"
    return "none"


def app_base(url, html, final):
    if re.search(r"/app/?$", final.rstrip("/")) or final.rstrip("/").endswith("/app"):
        return final.split("?")[0].rstrip("/")
    for match in re.findall(r"https?://[^\"']+/(?:nicgep|eprocure|epublish)/app", html):
        return match.rstrip("/")
    for match in re.findall(r"href=[\"']([^\"']*/(?:nicgep|eprocure|epublish)/app)", html):
        if match.startswith("http"):
            return match.rstrip("/")
        origin = re.match(r"https?://[^/]+", final)
        if origin:
            return origin.group(0) + (match if match.startswith("/") else "/" + match)
    return None


def page_facts(url):
    status, final, html, err = fetch(url)
    if not html:
        return {
            "url": url,
            "status": status,
            "final": final,
            "error": err,
            "family": "",
            "challenge": "error",
            "titles": False,
            "present": None,
            "past": None,
        }
    text = plain(html)
    present = first_int(text, [
        r"Total Tenders\s*:\s*([\d,]{1,12})",
        r"Total Bid\(s\)\s*:\s*([\d,]{1,12})",
    ])
    past = first_int(text, [
        r"Total Archived Tenders\s*:\s*([\d,]{1,12})",
        r"Archived Tenders[^0-9]{0,30}([\d,]{3,12})",
        r"Total Results\s*:\s*([\d,]{1,12})",
        r"Closed Tenders[^0-9]{0,30}([\d,]{3,12})",
    ])
    return {
        "url": url,
        "status": status,
        "final": final,
        "error": err,
        "family": family_of(html, final),
        "challenge": challenge_of(html, text),
        "titles": titles_visible(html, text),
        "present": present,
        "past": past,
        "html": html,
        "text": text,
    }


def classify(home, extras):
    pages = [home] + extras
    live = [page for page in pages if page.get("status") == 200]
    if any(page["titles"] for page in live):
        group = "titles-visible"
    elif any(page["challenge"] in {"captcha", "otp", "login"} for page in live):
        group = "challenge-required"
    elif home.get("challenge") == "error":
        group = "error"
    else:
        group = "empty-shell"
    present = next((page["present"] for page in live if page["present"] is not None), None)
    past = next((page["past"] for page in live if page["past"] is not None), None)
    if group == "challenge-required" and not any(page["titles"] for page in live):
        present = None
        past = None
    challenge = "none"
    for name in ("otp", "captcha", "login"):
        if any(page["challenge"] == name for page in live):
            challenge = name
            break
    family = home.get("family") or next((page["family"] for page in live if page.get("family")), "")
    return {
        "url": home["url"],
        "status": home.get("status"),
        "final": home.get("final"),
        "error": home.get("error") or "",
        "family": family,
        "challenge": challenge if group == "challenge-required" else (challenge if challenge != "none" else "none"),
        "group": group,
        "titles": any(page["titles"] for page in live),
        "present": present,
        "past": past if past is not None else "hidden",
        "present_label": present if present is not None else "hidden",
    }


def probe_one(url):
    home = page_facts(url)
    extras = []
    html = home.get("html") or ""
    if home.get("family") == "gepnic" or "nicgep" in html.lower() or app_base(url, html, home.get("final") or url):
        base = app_base(url, html, home.get("final") or url)
        if base:
            for query in (
                "?page=FrontEndLatestActiveTenders&service=page",
                "?page=FrontEndLatestActiveTendersOrgwise&service=page",
            ):
                extra = page_facts(base + query)
                extra.pop("html", None)
                extra.pop("text", None)
                extras.append(extra)
                if extra.get("challenge") == "captcha" and not extra.get("titles"):
                    break
    home.pop("html", None)
    home.pop("text", None)
    row = classify(home, extras)
    row["checked"] = [home["url"]] + [page["url"] for page in extras]
    return row


def main():
    rows = []
    with ThreadPoolExecutor(max_workers=6) as pool:
        futures = {pool.submit(probe_one, url): url for url in URLS}
        for future in as_completed(futures):
            row = future.result()
            rows.append(row)
            print(f"{row['group']:20} present={str(row['present_label']):>8} past={str(row['past']):>8} {row['url']}", flush=True)
    rows.sort(key=lambda row: URLS.index(row["url"]))
    with open("probe-results.json", "w", encoding="utf-8") as handle:
        json.dump(rows, handle, indent=2)
        handle.write("\n")
    print(f"wrote {len(rows)} rows")


if __name__ == "__main__":
    main()
