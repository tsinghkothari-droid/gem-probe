#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from html import unescape
from http.cookiejar import CookieJar
from pathlib import Path
from urllib.parse import urljoin, urlparse
from urllib.request import HTTPCookieProcessor, Request, build_opener


def text(value):
    return ' '.join(unescape(re.sub(r'<[^>]+>', ' ', value)).split())


def rows(body):
    body = re.sub(r'<!--[\s\S]*?-->', '', body)
    return [re.findall(r'<td\b[^>]*>([\s\S]*?)</td>', row, re.I)
            for row in re.findall(r'<tr\b[^>]*class="(?:even|odd)"[^>]*>([\s\S]*?)</tr>', body, re.I)]


def organisations(body, base):
    output = []
    for cells in rows(body):
        if len(cells) != 3:
            continue
        link = re.search(r'href="([^"]+)"', cells[2])
        count = text(cells[2]).replace(',', '')
        if link and count.isdigit():
            url = urljoin(base, unescape(link[1]))
            if urlparse(url).netloc == urlparse(base).netloc:
                output.append({'name': text(cells[1]), 'count': int(count), 'url': url})
    return output


def tenders(body, source, organisation):
    output = {}
    for cells in rows(body):
        if len(cells) < 6:
            continue
        match = re.search(r'20\d{2}_[A-Z][A-Z0-9]*_\d+(?:_\d+)*', text(cells[4]), re.I)
        link = re.search(r'<a\b[^>]*>([\s\S]*?)</a>', cells[4], re.I)
        if not match or not link:
            continue
        tender_id = match.group()
        parts = re.findall(r'\[([^\[\]]+)\]', text(cells[4]))
        output[tender_id] = {
            'id': tender_id, 'title': text(link[1]).strip('[] '),
            'reference': parts[-2] if len(parts) >= 2 else '',
            'organisation': organisation, 'buyer_chain': text(cells[5]),
            'published': text(cells[1]), 'closing': text(cells[2]),
            'opening': text(cells[3]), 'source_url': source,
        }
    return list(output.values())


def collect(url, max_orgs=2, organisation='', delay=1, timeout=30, transport='urllib'):
    result = {'source_url': url, 'checked_at': datetime.now(timezone.utc).isoformat(),
              'status': 'unresolved', 'scans': [], 'tenders': [], 'transport': transport}
    http = build_opener(HTTPCookieProcessor(CookieJar()))
    cookie_file = tempfile.NamedTemporaryFile(prefix='gem-probe-cookies-', delete=False)
    cookie_file.close()

    def fetch(target):
        if transport == 'curl':
            response = subprocess.run([
                'curl', '--fail', '--silent', '--show-error', '--location',
                '--proto', '=https', '--proto-redir', '=https',
                '--max-time', str(timeout), '--max-filesize', '16000000',
                '--cookie', cookie_file.name, '--cookie-jar', cookie_file.name,
                '--user-agent', 'gem-probe/0.1 (public organisation tender collector)',
                '--header', 'Accept-Language: en-IN', '--write-out', '\n%{url_effective}', target,
            ], capture_output=True)
            if response.returncode:
                raise ValueError(response.stderr.decode('utf-8', 'replace').strip()[:200])
            body, final_url = response.stdout.rsplit(b'\n', 1)
            if urlparse(final_url.decode()).netloc != urlparse(url).netloc:
                raise ValueError('Portal redirected to another host')
            return body.decode('utf-8', 'replace')
        request = Request(target, headers={
            'User-Agent': 'gem-probe/0.1 (public organisation tender collector)',
            'Accept-Language': 'en-IN',
        })
        with http.open(request, timeout=timeout) as response:
            if urlparse(response.geturl()).netloc != urlparse(url).netloc:
                raise ValueError('Portal redirected to another host')
            body = response.read(16_000_001)
            if len(body) > 16_000_000:
                raise ValueError('Response exceeds 16 MB limit')
            return body.decode('utf-8', 'replace')

    try:
        directory = fetch(url)
        orgs = organisations(directory, url)
        result['organisations_listed'] = len(orgs)
        result['advertised_active_tenders'] = sum(org['count'] for org in orgs)
        if not orgs:
            result['status'] = 'challenge_required' if 'captchaText' in directory else 'directory_not_found'
            result['directory_reports_no_active_tenders'] = 'No Active Tenders found' in text(directory)
            return result
        selected = [org for org in orgs if organisation.casefold() in org['name'].casefold()]
        if max_orgs:
            selected = selected[:max_orgs]
        result['organisations_selected'] = len(selected)
        seen = set()
        for org in selected:
            time.sleep(delay)
            scan = {'organisation': org['name'], 'advertised_count': org['count']}
            try:
                notices = tenders(fetch(org['url']), url, org['name'])
                scan['parsed_count'] = len(notices)
                scan['status'] = 'count_matches' if len(notices) == org['count'] else 'count_mismatch'
                for notice in notices:
                    if notice['id'] not in seen:
                        result['tenders'].append(notice)
                        seen.add(notice['id'])
            except Exception as error:
                scan.update(status='fetch_error', error=str(error)[:200])
            result['scans'].append(scan)
        result['status'] = 'records_returned' if result['tenders'] else 'no_records_returned'
        result['complete_active_snapshot'] = bool(selected) and len(selected) == len(orgs) and all(
            scan['status'] == 'count_matches' for scan in result['scans'])
    except Exception as error:
        result.update(status='fetch_error', error=str(error)[:200])
    finally:
        Path(cookie_file.name).unlink(missing_ok=True)
    return result


def main():
    parser = argparse.ArgumentParser(description='Collect GePNIC active tenders through public organisation listings.')
    parser.add_argument('--url', required=True)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--max-orgs', type=int, default=2, help='Default 2; use 0 to scan every organisation.')
    parser.add_argument('--organisation', default='')
    parser.add_argument('--transport', choices=('urllib', 'curl'), default='urllib')
    parser.add_argument('--timeout', type=int, default=30, help='Seconds per request; large organisations can need more time.')
    parser.add_argument('--query', default='', help='Filter collected titles, references and buyer chains locally.')
    args = parser.parse_args()
    if args.max_orgs < 0 or args.timeout < 1 or not args.url.startswith('https://'):
        parser.error('Use an HTTPS URL, a nonnegative organisation limit and a positive timeout.')
    report = collect(args.url, args.max_orgs, args.organisation, timeout=args.timeout, transport=args.transport)
    words = args.query.casefold().split()
    report['query'] = args.query
    report['matches'] = [notice for notice in report['tenders'] if all(
        word in ' '.join([notice['title'], notice['reference'], notice['buyer_chain']]).casefold() for word in words)]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({key: value for key, value in report.items() if key not in ('tenders', 'matches')}, indent=2))
    print(f"Returned {len(report['tenders'])} records; {len(report['matches'])} match the local query.")


if __name__ == '__main__':
    main()
