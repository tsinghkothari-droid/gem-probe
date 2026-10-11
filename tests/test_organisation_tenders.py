import unittest
from organisation_tenders import organisations,tenders

class ParserTests(unittest.TestCase):
 def test_mixed_case_id_and_commented_column(self):
  row='<tr class="even"><td>1</td><td>26-Aug-2026</td><td>14-Nov-2026</td><td>16-Nov-2026</td><td><a href="/view">[Design of ERP System]</a> [7/EDP/13815][2026_DoD_786434_1]</td><!-- <td>obsolete</td> --><td>Department of Defence||Armed Forces</td></tr>'
  notices=tenders(row+row,'https://example.test/app','Department of Defence')
  self.assertEqual(len(notices),1)
  self.assertEqual(notices[0]['id'],'2026_DoD_786434_1')
  self.assertEqual(notices[0]['title'],'Design of ERP System')
  self.assertEqual(notices[0]['reference'],'7/EDP/13815')
  self.assertEqual(notices[0]['buyer_chain'],'Department of Defence||Armed Forces')
 def test_organisation_link_keeps_query_and_rejects_other_host(self):
  row='<tr class="odd"><td>1</td><td>Agriculture</td><td><a href="/app?session=T&amp;sp=abc">7</a></td></tr>'
  orgs=organisations(row,'https://example.test/app')
  self.assertEqual(orgs[0]['url'],'https://example.test/app?session=T&sp=abc')
  self.assertEqual(orgs[0]['count'],7)
  self.assertEqual(organisations(row.replace('/app?','https://other.test/app?'),'https://example.test/app'),[])
 def test_non_record_table_is_not_a_tender(self):
  self.assertEqual(tenders('<tr class="odd"><td>No Active Tenders found.</td></tr>','https://example.test/app',''),[])

if __name__=='__main__':unittest.main()
