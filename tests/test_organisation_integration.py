import json
import io
import subprocess
import sys
import unittest
from unittest.mock import MagicMock, patch

from organisation_tenders import bihar_notices, collect, collect_bihar, main, restore_case, unlock_directory


class CaptchaTests(unittest.TestCase):
    def form(self, token='first'):
        return ('<form action="/app"><input name="tokenSecret" value="' + token + '">'
                '<input name="captchaText"><select name="tenderCategory">'
                '<option value="0">All</option><option selected value="3">Works</option></select>'
                '<input type="submit" name="Search" value="Search">'
                '<img id="captchaImage" src="data:image/png;base64,YWJj"></form>')

    def test_rejection_refreshes_hidden_fields_in_same_session(self):
        calls = []
        directory = '<tr class="odd"><td>1</td><td>Public Works</td><td><a href="/app?session=T">1</a></td></tr>'

        def fetch(url, fields=None):
            calls.append((url, fields))
            return 'Invalid Captcha!' + self.form('refreshed') if len(calls) == 1 else directory

        with patch('organisation_tenders.solve_image', side_effect=['ABC123', 'DEF456']):
            _, attempts = unlock_directory(self.form(), 'https://example.test/app', fetch, 'local')
        self.assertEqual([a['status'] for a in attempts], ['challenge_rejected', 'directory_returned'])
        self.assertEqual(calls[0][1]['tokenSecret'], 'first')
        self.assertEqual(calls[1][1]['tokenSecret'], 'refreshed')
        self.assertEqual(calls[1][1]['tenderCategory'], '3')
        self.assertEqual(calls[1][1]['Search'], 'Search')

    def test_empty_table_is_not_a_confirmed_zero(self):
        with patch('organisation_tenders.solve_image', return_value='ABC123'):
            _, attempts = unlock_directory(self.form(), 'https://example.test/app',
                                           lambda *args: 'No Active Tenders found.', 'local')
        self.assertEqual(attempts[0]['status'], 'empty_unverified')

    def test_case_restoration_handles_noise_without_changing_letters(self):
        self.assertEqual(restore_case('3yen6x', '3Yern6ix'), '3Yen6x')
        self.assertEqual(restore_case('x2uq1e', 'x2Ud1E'), 'x2uq1e')

    def test_off_host_form_action_is_rejected(self):
        with patch('organisation_tenders.solve_image') as solver:
            _, attempts = unlock_directory(self.form().replace('action="/app"', 'action="https://other.test/app"'),
                                           'https://example.test/app', None, 'local')
        solver.assert_not_called()
        self.assertEqual(attempts[0]['status'], 'invalid_form_action')


class CoverageTests(unittest.TestCase):
    def test_challenge_does_not_report_zero_active_tenders(self):
        response = MagicMock()
        response.__enter__.return_value = response
        response.read.return_value = b'<input name="captchaText">'
        response.geturl.return_value = 'https://example.test/app'
        with patch('organisation_tenders.build_opener') as client:
            client.return_value.open.return_value = response
            report = collect('https://example.test/app', delay=0)
        self.assertEqual(report['status'], 'challenge_required')
        self.assertIsNone(report['advertised_active_tenders'])
        self.assertFalse(report['complete_active_snapshot'])

    def test_limited_buyer_filter_is_still_partial(self):
        directory = ''.join('<tr class="odd"><td>1</td><td>Public ' + name + '</td><td><a href="/app?sp=' + name + '">1</a></td></tr>'
                            for name in ['One', 'Two'])
        notice = '<tr class="even"><td>1</td><td>Date</td><td>Date</td><td>Date</td><td><a>[Title]</a>[Ref][2026_PWD_1_1]</td><td>Public One</td></tr>'

        def response(body):
            value = MagicMock()
            value.__enter__.return_value = value
            value.read.return_value = body.encode()
            value.geturl.return_value = 'https://example.test/app'
            return value

        with patch('organisation_tenders.build_opener') as client:
            client.return_value.open.side_effect = [response(directory), response(notice)]
            result = collect('https://example.test/app', max_orgs=1, organisation='Public', delay=0)
        self.assertEqual(len(result['tenders']), 1)
        self.assertFalse(result['complete_selected_snapshot'])
        self.assertFalse(result['complete_active_snapshot'])

    def test_invalid_url_does_not_abort_other_hosts(self):
        portals = [{'host': 'bad.test', 'url': 'http://bad.test'},
                   {'host': 'good.test', 'url': 'https://good.test'}]
        output = io.StringIO()
        with patch('sys.argv', ['collector', '--batch', '--out', '-']), \
             patch('sys.stdin', io.StringIO(json.dumps(portals))), \
             patch('sys.stdout', output), patch('sys.stderr', io.StringIO()), \
             patch('organisation_tenders.collect', return_value={'status': 'records_returned', 'tenders': []}) as collect_mock:
            main()
        reports = json.loads(output.getvalue())
        self.assertEqual([r['status'] for r in reports], ['configuration_error', 'records_returned'])
        collect_mock.assert_called_once()

    def test_every_configured_portal_uses_https(self):
        from pathlib import Path
        portals = json.loads(Path('gepnic-portals.json').read_text())
        self.assertTrue(all(p['url'].startswith('https://') for p in portals))

    def test_batch_stdout_is_json(self):
        result = subprocess.run([sys.executable, 'organisation_tenders.py', '--batch', '--out', '-'],
                                input='[]', capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout), [])


class BiharTests(unittest.TestCase):
    def test_nested_buyer_names_dates_and_duplicate_ids(self):
        buyers = [{'organizationId': 1, 'organizationName': 'Department',
                   'childOrgList': [{'organizationId': 2, 'organizationName': 'Division'}]}]
        record = {'currenttenderid': 42, 'currentdescription': '<b>Road</b> &amp; bridge',
                  'currentorgid': 1, 'currentdeptid': 2, 'currentbidEndDate': 0}
        notices = bihar_notices([record, record, {}], buyers, 'https://example.test/list')
        self.assertEqual(len(notices), 1)
        self.assertEqual(notices[0]['id'], 'BIHAR_42')
        self.assertEqual(notices[0]['buyer_chain'], 'Department || Division')
        self.assertEqual(notices[0]['title'], 'Road & bridge')
        self.assertEqual(notices[0]['closing'], '1970-01-01T05:30:00+05:30')

    def test_count_mismatch_keeps_records_but_marks_partial(self):
        responses = [b'<input id="Authorization" value="">',
                     b'<input id="Authorization" value="anonymous-token">',
                     b'[{"currenttenderid":42,"currentdescription":"Road","currentorgid":1,"currentdeptid":2}]',
                     b'[{"organizationId":1,"organizationName":"Department"}]',
                     b'[{"organizationId":2,"organizationName":"Division"}]',
                     b'{"activeTenderCount":2}']
        with patch('organisation_tenders.subprocess.run') as run:
            run.side_effect = [subprocess.CompletedProcess([], 0, body, b'') for body in responses]
            report = collect_bihar()
        self.assertEqual(len(report['tenders']), 1)
        self.assertFalse(report['complete_active_snapshot'])
        self.assertEqual(report['scans'][0]['status'], 'count_mismatch')
        self.assertEqual(report['tenders'][0]['buyer_chain'], 'Department || Division')

    def test_metadata_failure_retains_fetched_tenders(self):
        responses = [b'<input id="Authorization" value="">',
                     b'<input id="Authorization" value="anonymous-token">',
                     b'[{"currenttenderid":42,"currentdescription":"Road"}]', b'[]']
        with patch('organisation_tenders.subprocess.run') as run:
            run.side_effect = ([subprocess.CompletedProcess([], 0, body, b'') for body in responses] +
                               [subprocess.CompletedProcess([], 22, b'', b'HTTP 503')])
            report = collect_bihar()
        self.assertEqual(len(report['tenders']), 1)
        self.assertEqual(report['status'], 'fetch_error')
        self.assertFalse(report['complete_active_snapshot'])
