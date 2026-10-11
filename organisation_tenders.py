#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
import tempfile
import time
from datetime import datetime, timezone, timedelta
from html import unescape
from html.parser import HTMLParser
from http.cookiejar import CookieJar
from pathlib import Path
from urllib.parse import urlencode, urljoin, urlparse
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



class CaptchaForm(HTMLParser):
    def __init__(self):
        super().__init__()
        self.forms = []
        self.form = None
        self.select = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'form':
            self.form = {'action': attrs.get('action', ''), 'fields': {}, 'image': None}
            self.forms.append(self.form)
        if self.form is None:
            return
        if tag == 'input' and attrs.get('name') and 'disabled' not in attrs:
            kind = attrs.get('type', 'text').lower()
            if kind not in ('button', 'reset', 'checkbox', 'radio') or 'checked' in attrs:
                self.form['fields'][attrs['name']] = attrs.get('value', '')
        elif tag == 'select':
            self.select = attrs.get('name')
        elif tag == 'option' and self.select:
            if self.select not in self.form['fields'] or 'selected' in attrs:
                self.form['fields'][self.select] = attrs.get('value', '')
        elif tag == 'img' and attrs.get('id') == 'captchaImage':
            self.form['image'] = attrs.get('src')

    def handle_endtag(self, tag):
        if tag == 'form':
            self.form = None
        elif tag == 'select':
            self.select = None


def collect(url, max_orgs=2, organisation='', delay=1, timeout=30, transport='urllib'):
    result = {'source_url': url, 'checked_at': datetime.now(timezone.utc).isoformat(),
              'status': 'unresolved', 'scans': [], 'tenders': [], 'transport': transport,
              'organisation_filter': organisation, 'complete_active_snapshot': False,
              'complete_selected_snapshot': False}
    http = build_opener(HTTPCookieProcessor(CookieJar()))
    cookie_file = tempfile.NamedTemporaryFile(prefix='gem-probe-cookies-', delete=False)
    cookie_file.close()

    def fetch(target, fields=None):
        payload = urlencode(fields).encode() if fields is not None else None
        if transport == 'curl':
            command = [
                'curl', '--fail', '--silent', '--show-error', '--location',
                '--proto', '=https', '--proto-redir', '=https',
                '--max-time', str(timeout), '--max-filesize', '16000000',
                '--cookie', cookie_file.name, '--cookie-jar', cookie_file.name,
                '--user-agent', 'gem-probe/0.1 (public organisation tender collector)',
                '--header', 'Accept-Language: en-IN', '--write-out', '\n%{url_effective}', target,
            ]
            if payload is not None:
                command += ['--header', 'Content-Type: application/x-www-form-urlencoded', '--referer', url, '--data-binary', '@-']
            response = subprocess.run(command, input=payload, capture_output=True)
            if response.returncode:
                raise ValueError(response.stderr.decode('utf-8', 'replace').strip()[:200])
            body, final_url = response.stdout.rsplit(b'\n', 1)
            if urlparse(final_url.decode()).netloc != urlparse(url).netloc:
                raise ValueError('Portal redirected to another host')
            return body.decode('utf-8', 'replace')
        request = Request(target, data=payload, headers={
            'User-Agent': 'gem-probe/0.1 (public organisation tender collector)',
            'Accept-Language': 'en-IN', 'Referer': url,
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
        result['advertised_active_tenders'] = sum(org['count'] for org in orgs) if orgs else None
        if not orgs:
            result['status'] = 'challenge_required' if 'captchaText' in directory else 'directory_not_found'
            result['directory_reports_no_active_tenders'] = 'No Active Tenders found' in text(directory)
            return result
        selected = [org for org in orgs if organisation.casefold() in org['name'].casefold()]
        result['organisations_matching'] = len(selected)
        if max_orgs:
            selected = selected[:max_orgs]
        result['organisations_selected'] = len(selected)
        if not selected:
            result['status'] = 'organisation_not_found'
            return result
        consecutive_errors = 0
        seen = set()
        for org in selected:
            time.sleep(delay)
            scan = {'organisation': org['name'], 'advertised_count': org['count']}
            try:
                notices = tenders(fetch(org['url']), url, org['name'])
                consecutive_errors = 0
                scan['parsed_count'] = len(notices)
                scan['status'] = 'count_matches' if len(notices) == org['count'] else 'count_mismatch'
                for notice in notices:
                    if notice['id'] not in seen:
                        result['tenders'].append(notice)
                        seen.add(notice['id'])
            except Exception as error:
                consecutive_errors += 1
                scan.update(status='fetch_error', error=str(error)[:200])
            result['scans'].append(scan)
            if consecutive_errors >= 3:
                result['stopped_after_errors'] = True
                break
        result['status'] = 'records_returned' if result['tenders'] else 'no_records_returned'
        result['complete_selected_snapshot'] = len(result['scans']) == result['organisations_matching'] and all(
            scan['status'] == 'count_matches' for scan in result['scans'])
        result['complete_active_snapshot'] = len(selected) == len(orgs) and result['complete_selected_snapshot']
    except Exception as error:
        result.update(status='fetch_error', error=str(error)[:200])
    finally:
        Path(cookie_file.name).unlink(missing_ok=True)
    return result



class PageToken(HTMLParser):
    def __init__(self):
        super().__init__()
        self.value = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'input' and attrs.get('id') == 'Authorization':
            self.value = attrs.get('value', '')


def bihar_notices(records, buyers, source):
    names = {}

    def visit(items, parents=()):
        for item in items:
            name = item.get('organizationName') or ''
            chain = parents + (name,) if name and name != '-' else parents
            names[item['organizationId']] = ' || '.join(chain)
            visit(item.get('childOrgList') or [], chain)

    visit(buyers)

    def date(value):
        if value is None:
            return ''
        return datetime.fromtimestamp(value / 1000, timezone(timedelta(hours=5, minutes=30))).isoformat()

    notices = {}
    for record in records:
        tender_id = record.get('currenttenderid')
        title = record.get('currentdescription')
        if not tender_id or not title:
            continue
        buyer = names.get(record.get('currentdeptid')) or names.get(record.get('currentorgid')) or ''
        identifier = 'BIHAR_' + str(tender_id)
        notices[identifier] = {
            'id': identifier, 'title': text(title), 'reference': record.get('currenttenderrefno') or '',
            'organisation': buyer, 'buyer_chain': buyer,
            'published': date(record.get('currentTenderPublishDate')),
            'closing': date(record.get('currentbidEndDate')), 'opening': date(record.get('currentbidOpenDate')),
            'source_url': source,
        }
    return list(notices.values())


def collect_bihar(timeout=30):
    base = 'https://eproc2.bihar.gov.in/EPSV2Web'
    source = base + '/openarea/tenderListingPage.action'
    result = {'host': 'eproc2.bihar.gov.in', 'kind': 'bihar', 'source_url': source,
              'checked_at': datetime.now(timezone.utc).isoformat(), 'status': 'unresolved',
              'scans': [], 'tenders': [], 'complete_active_snapshot': False}
    with tempfile.TemporaryDirectory(prefix='gem-probe-bihar-') as directory:
        cookie = Path(directory) / 'cookies.txt'
        cookie.touch()
        headers = Path(directory) / 'headers.txt'

        def fetch(path, data=None, token=None, content_type='application/json;charset=utf-8;'):
            fields = ['Content-Type: ' + content_type, 'Auth-Token: X-Requested-With']
            if token is not None:
                fields.append('Authorization: ' + token)
            headers.write_text('\n'.join(fields) + '\n')
            headers.chmod(0o600)
            command = ['curl', '--fail', '--silent', '--show-error', '--max-time', str(timeout),
                       '--max-filesize', '16000000', '--cookie', str(cookie), '--cookie-jar', str(cookie),
                       '--header', '@' + str(headers), base + path]
            if data is not None:
                command += ['--data-binary', '@-']
            response = subprocess.run(command, input=data, capture_output=True)
            if response.returncode:
                raise ValueError(response.stderr.decode('utf-8', 'replace').strip()[:200])
            return response.stdout.decode('utf-8', 'replace')

        try:
            home = PageToken()
            home.feed(fetch('/'))
            if home.value is None:
                raise ValueError('Anonymous navigation form was not found')
            listing = PageToken()
            listing.feed(fetch('/openarea/tenderListingPage.action',
                               urlencode({'Authorization': home.value}).encode(),
                               content_type='application/x-www-form-urlencoded'))
            if not listing.value:
                raise ValueError('Public listing did not issue an anonymous page token')
            records = json.loads(fetch('/rest/openarea/getTenderList', b'{}', listing.value))
            buyers = json.loads(fetch('/rest/organization/loadAllDeptWithParent?orgId=0', b'', listing.value))
            if not isinstance(records, list) or not isinstance(buyers, list):
                raise ValueError('Unexpected Bihar tender or buyer response format')
            result['tenders'] = bihar_notices(records, buyers, source)
            roots = {record.get('currentorgid') for record in records}
            for buyer in buyers:
                if buyer['organizationId'] in roots:
                    children = json.loads(fetch('/rest/organization/getChildOrgUnderProvidedOrgID?orgId=' +
                                                str(buyer['organizationId']), b'', listing.value))
                    if not isinstance(children, list):
                        raise ValueError('Unexpected Bihar department response format')
                    buyer['childOrgList'] = children
            result['tenders'] = bihar_notices(records, buyers, source)
            counts = json.loads(fetch('/rest/tender/getStatistics', b'', listing.value))
            count = counts.get('activeTenderCount')
            complete = isinstance(count, int) and len(records) == len(result['tenders']) == count
            result['advertised_active_tenders'] = count
            result['complete_active_snapshot'] = complete
            result['status'] = 'records_returned' if result['tenders'] else ('verified_empty' if complete else 'no_records_returned')
            result['scans'] = [{'organisation': 'Bihar public active-tender list', 'advertised_count': count or 0,
                               'parsed_count': len(result['tenders']), 'status': 'count_matches' if complete else 'count_mismatch'}]
        except Exception as error:
            result.update(status='fetch_error', error=str(error)[:200])
    return result


class PublicSession:
    def __init__(self, timeout):
        self.timeout = timeout
        self.directory = tempfile.TemporaryDirectory(prefix='gem-probe-public-')
        self.cookie = Path(self.directory.name) / 'cookies'
        self.headers = Path(self.directory.name) / 'headers'

    def close(self):
        self.directory.cleanup()

    def fetch(self, url, data=None, content_type='application/json'):
        command = ['curl', '--fail', '--silent', '--show-error', '--max-time', str(self.timeout),
                   '--max-filesize', '16000000', '--proto', '=https',
                   '--cookie', str(self.cookie), '--cookie-jar', str(self.cookie),
                   '--dump-header', str(self.headers), url]
        if data is not None:
            command += ['--header', 'Content-Type: ' + content_type, '--data-binary', '@-']
        response = subprocess.run(command, input=data, capture_output=True)
        if response.returncode:
            raise ValueError(response.stderr.decode('utf-8', 'replace').strip()[:200])
        headers = self.headers.read_text(errors='replace')
        return response.stdout.decode('utf-8', 'replace'), headers


def public_notice(identifier, title, buyer, reference, closing, published, source, opening=''):
    return {'id': identifier, 'title': text(title), 'reference': reference or '',
            'organisation': buyer or '', 'buyer_chain': buyer or '', 'closing': closing or '',
            'published': published or '', 'opening': opening or '', 'source_url': source}


def karnataka_notices(records, source, category='UNKNOWN'):
    def date(value):
        if not value:
            return ''
        try:
            return datetime.strptime(value, '%d-%m-%Y %H:%M:%S').isoformat() + '+05:30'
        except ValueError:
            return value
    return [public_notice('KPPP_' + str(row.get('category') or category) + '_' + str(row['id']), row['title'],
                          ' || '.join(filter(None, [row.get('deptName'), row.get('locationName')])),
                          row.get('tenderNumber'), date(row.get('tenderClosureDate')),
                          date(row.get('publishedDate')), source)
            for row in records if row.get('id') and row.get('title')]


def shpp_notices(records, source):
    notices = []
    for row in records:
        identifier = row.get('ubnNumber') or row.get('tenderRef')
        if not identifier or not row.get('titleName'):
            continue
        notices.append(public_notice('RAJ_' + str(identifier), row['titleName'],
                       ' || '.join(filter(None, [row.get('departmentName'), row.get('officeName')])),
                       row.get('tenderRef'), row.get('bidSubmissionEndDate'), row.get('publishedDate'),
                       source, row.get('bidOpeningDate')))
    return notices


class HomepageNotices(HTMLParser):
    def __init__(self, prefix, source):
        super().__init__()
        self.prefix, self.source = prefix, source
        self.notices = {}
        self.current = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'a' and re.search(r'\bviewtender\(\d+\)', attrs.get('onclick', ''), re.I):
            title = attrs.get('title', '')
            if title.startswith('(') and title.endswith(')'):
                title = title[1:-1]
            self.current = {'title': title, 'id': ''}

    def handle_data(self, data):
        if self.current is not None:
            self.current['id'] += data

    def handle_endtag(self, tag):
        if tag == 'a' and self.current is not None:
            identifier = self.current['id'].strip()
            if identifier.isdigit() and self.current['title']:
                self.notices[identifier] = public_notice(self.prefix + identifier, self.current['title'],
                                                        '', '', '', '', self.source)
            self.current = None


def homepage_notices(body, prefix, source):
    parser = HomepageNotices(prefix, source)
    parser.feed(re.sub(r'<!--[\s\S]*?-->', '', body))
    return list(parser.notices.values())


def collect_public(portal, timeout=30, pages=2):
    source, kind = portal['url'], portal['kind']
    result = {'host': portal['host'], 'kind': kind, 'source_url': source,
              'checked_at': datetime.now(timezone.utc).isoformat(), 'status': 'unresolved',
              'scope': '', 'scans': [], 'tenders': [], 'complete_active_snapshot': False,
              'complete_selected_snapshot': False, 'advertised_active_tenders': None}
    session = PublicSession(timeout)
    seen = set()

    def retain(notices):
        fresh = 0
        for notice in notices:
            if notice['id'] not in seen:
                result['tenders'].append(notice)
                seen.add(notice['id'])
                fresh += 1
        return fresh

    try:
        if kind in ('ap_home', 'telangana_home'):
            result['scope'] = 'Public homepage notices only, including any displayed corrigenda; buyer names and dates not extracted'
            body, _ = session.fetch(source)
            form = CaptchaForm()
            form.feed(body)
            fields = next((f['fields'] for f in form.forms if 'CSRFToken' in f['fields']), None)
            if fields is None:
                raise ValueError('Anonymous entry form was not found')
            body, _ = session.fetch(urljoin(source, 'login.html'), urlencode(fields).encode(),
                                    'application/x-www-form-urlencoded')
            retain(homepage_notices(body, 'AP_' if kind == 'ap_home' else 'TS_', urljoin(source, 'login.html')))
            result['scans'].append({'organisation': 'Public homepage', 'advertised_count': None,
                                    'parsed_count': len(result['tenders']), 'status': 'sample_only'})
        else:
            if kind == 'karnataka':
                base = 'https://kppp.karnataka.gov.in/supplier-registration-service/v1/api/portal-service/'
                routes = [('GOODS', base + 'search-eproc-tenders'), ('WORKS', base + 'works/search-eproc-tenders'),
                          ('SERVICES', base + 'services/search-eproc-tenders')]
                result['scope'] = 'Published OPEN tenders: goods, works, services; bounded pages per category'
            else:
                base = 'https://shpp.rajasthan.gov.in/data/tenderuploadservice/api/v1/alltenders/'
                routes = [('SHPP', base + 'publishedTenders'), ('SPPP', base + 'publishedTendersSppp')]
                result['scope'] = 'Published SHPP and SPPP listings; SPPP full total is unverified'
            for label, endpoint in routes:
                scan = {'organisation': label, 'advertised_count': None, 'parsed_count': 0, 'status': 'page_limit'}
                local_ids = set()
                result['scans'].append(scan)
                try:
                    for page in range(pages):
                        if page:
                            time.sleep(1)
                        if kind == 'karnataka':
                            url = endpoint + '?' + urlencode({'page': page, 'size': 100, 'order-by-tender-publish': 'true'})
                            payload = {'category': label, 'status': 'PUBLISHED', 'tenderType': 'OPEN'}
                        else:
                            url, payload = endpoint, {'pageNumber': page, 'pageSize': 100}
                        body, headers = session.fetch(url, json.dumps(payload).encode())
                        data = json.loads(body)
                        if kind == 'karnataka':
                            records = data
                            match = re.search(r'^x-total-count:\s*(\d+)', headers, re.I | re.M)
                            total = int(match[1]) if match else None
                            notices = karnataka_notices(records, source, label) if isinstance(records, list) else []
                        else:
                            if data.get('msgStatus') != 'S' or (label == 'SPPP' and not data.get('result', {}).get('spppIncluded')):
                                raise ValueError('Public listing reported a failed or deferred response')
                            records = data['result'].get('publishedTenders')
                            total = data['result'].get('shppRecordCount') if label == 'SHPP' else None
                            notices = shpp_notices(records, source) if isinstance(records, list) else []
                        if not isinstance(records, list):
                            raise ValueError('Unexpected public listing response format')
                        scan['advertised_count'] = total
                        old_count = len(local_ids)
                        local_ids.update(row['id'] for row in notices)
                        retain(notices)
                        scan['parsed_count'] = len(local_ids)
                        if len(notices) != len(records):
                            scan['status'] = 'parse_mismatch'
                            break
                        if total is not None and len(local_ids) == total:
                            scan['status'] = 'count_matches'
                            break
                        if not records or len(local_ids) == old_count:
                            scan['status'] = 'count_mismatch' if total is not None else 'unverified_total'
                            break
                except Exception as error:
                    scan.update(status='fetch_error', error=str(error)[:200])
            result['complete_selected_snapshot'] = bool(result['scans']) and all(
                s['status'] == 'count_matches' for s in result['scans'])
        result['status'] = 'records_returned' if result['tenders'] else 'no_records_returned'
    except Exception as error:
        result.update(status='fetch_error', error=str(error)[:200])
    finally:
        session.close()
    return result


def main():
    parser = argparse.ArgumentParser(description='Collect public active tenders from GePNIC and Bihar.')
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--url')
    source.add_argument('--batch', action='store_true', help='Read portal configuration JSON from stdin.')
    parser.add_argument('--out', required=True, help='JSON output path, or - for stdout.')
    parser.add_argument('--max-orgs', type=int, default=2, help='Default 2; use 0 to scan every organisation.')
    parser.add_argument('--organisation', default='')
    parser.add_argument('--transport', choices=('urllib', 'curl'), default='urllib')
    parser.add_argument('--timeout', type=int, default=30, help='Seconds per request; large organisations can need more time.')
    parser.add_argument('--listing-pages', type=int, default=2, help='Pages per category for public JSON listings.')
    parser.add_argument('--query', default='', help='Filter collected titles, references and buyer chains locally.')
    args = parser.parse_args()
    if args.max_orgs < 0 or args.timeout < 1 or args.listing_pages < 1 or (args.url and not args.url.startswith('https://')):
        parser.error('Use an HTTPS URL, a nonnegative organisation limit and a positive timeout.')
    def run_one(portal):
        if not portal.get('url', '').startswith('https://'):
            report = {'source_url': portal.get('url', ''), 'checked_at': datetime.now(timezone.utc).isoformat(),
                      'status': 'configuration_error', 'error': 'Portal URL must use HTTPS',
                      'complete_active_snapshot': False, 'scans': [], 'tenders': []}
        elif portal.get('kind') in ('karnataka', 'shpp', 'ap_home', 'telangana_home'):
            report = collect_public(portal, timeout=args.timeout, pages=args.listing_pages)
        elif portal.get('kind') == 'bihar':
            report = collect_bihar(timeout=args.timeout)
        else:
            report = collect(portal['url'], args.max_orgs, portal.get('organisation', ''),
                             timeout=args.timeout, transport=args.transport)
        report['host'] = portal['host']
        words = args.query.casefold().split()
        report['query'] = args.query
        if words:
            report['matches'] = [notice for notice in report['tenders'] if all(
                word in ' '.join([notice['title'], notice['reference'], notice['buyer_chain']]).casefold()
                for word in words)]
        print(f"{portal['host']}: {report['status']}, {len(report['tenders'])} records", file=sys.stderr, flush=True)
        return report

    if args.batch:
        portals = json.load(sys.stdin)
        if not isinstance(portals, list) or any(not isinstance(p, dict) or not isinstance(p.get('host'), str)
                                                or not isinstance(p.get('url'), str) for p in portals):
            parser.error('Batch input must be a list of portal configurations with host and URL strings.')
        with ThreadPoolExecutor(max_workers=4) as pool:
            report = list(pool.map(run_one, portals))
    else:
        report = run_one({'host': urlparse(args.url).hostname, 'url': args.url, 'organisation': args.organisation})
    encoded = json.dumps(report, ensure_ascii=False, indent=2) + '\n'
    if args.out == '-':
        sys.stdout.write(encoded)
    else:
        path = Path(args.out)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(encoded)



if __name__ == '__main__':
    main()
