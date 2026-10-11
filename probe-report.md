# Portal probe, 11 Oct 2026

Public page reads only. No captcha, OTP, or login was submitted. Full rows are in `probe-results.json`.

## Counts the pages printed

Present means active or live. Past means awarded, archived, or result.

| Feed | Present | Past |
| --- | ---: | ---: |
| GeM BidPlus `ongoing_bids` / `bid_awarded` | 46,049 | 5,744,348 |
| CPPP state list `mmpdata` | 54,759 | hidden |
| CPPP central list `cpppdata` | 31,070 | hidden |
| CPPP GeM copy `gemdata` | 34,552 | hidden |
| CPPP high value | 575 | hidden |
| CPPP closing today | 11 | hidden |
| CPPP result of tenders | hidden | hidden |

GeM is the only feed that printed a past total. The awarded figure is the `bid_awarded` count from the public BidPlus endpoint.

ePublish shows a few current notices on the home page. The number 1,446,827 on that site is the visitor counter. Its active-tender search and its archive search ask for a captcha. The archive page says "No Archived Tenders found" before a search.

## What the 58 portals did

- **47** ask for a captcha or an OTP before they show a tender total. That includes the state GePNIC sites, defence, PMGSY, Coal India, BHEL, NTPC, Indian Oil, and the other PSU hosts.
- **Titles with a count:** the CPPP lists above, and GeM BidPlus.
- **Empty shell:** Andhra Pradesh, Karnataka KPPP, Rajasthan SHPP, Telangana, Dadra and Nagar Haveli notice list, IREPS home, NBCC.
- **No response:** `gepnic.gov.in` and `www.nprocure.com` timed out. `tender.nprocure.com` answered and asked for a captcha and an OTP. `tenders.wb.gov.in` failed TLS. `wbtenders.gov.in` asks for a captcha.

## Navoday match on the newest 5 pages

`cargo test -- --ignored coverage` on 11 Oct 2026. A low rate is a result. The build stays green.

CPPP states: 50 scanned, 0 kept, 0.0% match. Every keep term hit 0. Uttar Pradesh 40, Rajasthan 10. Gujarat, Karnataka, Maharashtra, and West Bengal did not appear. Published today: 50.

CPPP central: 45 scanned, 5 kept, 11.1% match. `brick work` hit 5. The other seven keep terms hit 0. None of the six state names appeared in the organisation column. Published today: 0.

Gates:

- Detail link: fail, no schedule text. The list page was the referer. The body was not "Invalid Url" and it had no schedule of quantities.
- IREPS quick-search: fail, no rows. The page says "No Results" and shows no tender id. No search was posted.
- IREPS advance-search: fail, no rows. Same result.
- CPWD script: pass, names no tender URL. `temp.test.js` links to crypto notes, not a tender list.

No IREPS reader and no CPWD reader. The CPPP reader stays because the central list kept five Navoday titles. The state list did not, and the detail page has no schedule text, so the watcher uses titles only.
