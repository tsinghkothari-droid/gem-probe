import json
import unittest
from unittest.mock import patch

from organisation_tenders import collect_public, homepage_notices, karnataka_notices, shpp_notices


class PublicListingTests(unittest.TestCase):
    def test_homepage_ignores_comments_and_preserves_title_parentheses(self):
        notice = '<a onclick="viewtender(12)" title="(Build drain (4th call))">123</a>'
        rows = homepage_notices('<!--' + notice.replace('123', '999') + '-->' + notice + notice,
                                'AP_', 'https://example.test')
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]['id'], 'AP_123')
        self.assertEqual(rows[0]['title'], 'Build drain (4th call)')
        self.assertEqual(rows[0]['buyer_chain'], '')

    def test_karnataka_buyer_and_date_mapping(self):
        row = {'id': 1, 'title': 'Build school', 'deptName': 'Education', 'locationName': 'Division',
               'tenderNumber': 'REF/1', 'tenderClosureDate': '21-10-2026 17:30:00'}
        notice = karnataka_notices([row], 'https://example.test')[0]
        self.assertEqual(notice['closing'], '2026-10-21T17:30:00+05:30')
        self.assertEqual(notice['buyer_chain'], 'Education || Division')

    def test_karnataka_ids_are_distinct_across_categories(self):
        row = {'id': 1, 'title': 'Tender'}
        goods = karnataka_notices([row], 'https://example.test', 'GOODS')[0]
        works = karnataka_notices([row], 'https://example.test', 'WORKS')[0]
        self.assertNotEqual(goods['id'], works['id'])

    def test_shpp_missing_identifiers_do_not_become_records(self):
        rows = shpp_notices([{'titleName': 'No ID'}, {'ubnNumber': 'R/1', 'titleName': 'Road',
                              'tenderRef': 'REF', 'departmentName': 'PWD'}], 'https://example.test')
        self.assertEqual([row['id'] for row in rows], ['RAJ_R/1'])

    def test_karnataka_repeated_pages_remain_partial(self):
        body = json.dumps([{'id': 1, 'title': 'School'}])
        portal = {'host': 'kppp.karnataka.gov.in', 'url': 'https://kppp.karnataka.gov.in/', 'kind': 'karnataka'}
        with patch('organisation_tenders.PublicSession.fetch', return_value=(body, 'X-Total-Count: 2')), \
             patch('organisation_tenders.time.sleep'):
            report = collect_public(portal, pages=2)
        self.assertEqual(len(report['tenders']), 3)
        self.assertFalse(report['complete_selected_snapshot'])
        self.assertTrue(all(s['status'] == 'count_mismatch' for s in report['scans']))

    def test_failed_later_category_keeps_earlier_records(self):
        body = json.dumps([{'id': 1, 'title': 'School'}])
        portal = {'host': 'kppp.karnataka.gov.in', 'url': 'https://kppp.karnataka.gov.in/', 'kind': 'karnataka'}
        with patch('organisation_tenders.PublicSession.fetch',
                   side_effect=[(body, 'X-Total-Count: 1'), ValueError('timeout'), ValueError('timeout')]):
            report = collect_public(portal, pages=1)
        self.assertEqual(len(report['tenders']), 1)
        self.assertEqual(report['scans'][1]['status'], 'fetch_error')
        self.assertFalse(report['complete_selected_snapshot'])

    def test_sppp_page_count_does_not_prove_complete_coverage(self):
        data = {'msgStatus': 'S', 'result': {'shppRecordCount': 1, 'spppIncluded': True, 'totalRecords': 1,
                'publishedTenders': [{'ubnNumber': 'R/1', 'titleName': 'Road'}]}}
        portal = {'host': 'shpp.rajasthan.gov.in', 'url': 'https://shpp.rajasthan.gov.in/', 'kind': 'shpp'}
        with patch('organisation_tenders.PublicSession.fetch', return_value=(json.dumps(data), '')):
            report = collect_public(portal, pages=1)
        self.assertEqual(report['scans'][0]['status'], 'count_matches')
        self.assertIsNone(report['scans'][1]['advertised_count'])
        self.assertFalse(report['complete_selected_snapshot'])
