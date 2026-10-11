# Automated tender collection: verified findings

Checked on 11 October 2026.

[Certain] Automated tender collection worked on **43 of 44 tested GePNIC hosts**. The collector fetched the public organisation directory, retained the session cookies, and followed each organisation's count link. It filtered keywords locally. It did not submit a CAPTCHA on this route. See the official URLs and results below, accessed 11 October 2026, and `organisation-route-results.json` for the records.

[Certain] The trial returned **2,533 records**, representing **2,338 distinct tender IDs** across the sampled hosts. Four company domains returned the same BEML records. A working domain therefore does not establish separate company coverage. The JSON evidence preserves each source URL and buyer chain. Accessed 11 October 2026.

[Certain] Each sampled organisation's parsed record count matched its directory count. Most hosts were sampled with one organisation. Six retests used up to two organisations. These counts do not establish complete portal coverage. See the per-host scans in the JSON evidence and the official URLs below, accessed 11 October 2026.

[Certain] A separate full [Arunachal scan](https://arunachaltenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) read both listed organisations and returned both advertised records. Its local search for `road` returned zero matches. That is a verified zero for this active snapshot only. See `arunachal-complete-trial.json`. Accessed 11 October 2026.

[Certain] The [Mizoram directory](https://mizoramtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) requests a CAPTCHA before listing organisations. It also displays “No Active Tenders found.” The collector marks it `challenge_required`; that empty table does not establish a zero tender count. Accessed 11 October 2026.

[NOT FOUND] Reliable automatic submission of the protected keyword search remains unverified. Four Rajasthan images were tested with local OCR. Three predictions failed the form's six-character requirement. One earlier submission failed a required Tender Type field check. No successful protected search was observed. The corrected prototype still produced invalid predictions. Source: live trials against the [Rajasthan advanced search](https://eproc.rajasthan.gov.in/nicgep/app?page=FrontEndAdvancedSearch&service=page), accessed 11 October 2026.

[Certain] The [Pipeworx source](https://github.com/pipeworx-io/mcp-india-tenders/blob/main/src/index.ts) documents the organisation route and session handling. Its README supplied the research lead. The delivered collector was written here and tested directly against the portals. Accessed 11 October 2026.

[Certain] [ddddocr](https://github.com/sml2h3/ddddocr) offers local image recognition. [2Captcha](https://2captcha.com/api-docs/normal-captcha) offers an image-to-text service through an API. Its [published price](https://2captcha.com/pricing) is $0.50–$1 per 1,000 normal image CAPTCHAs. No paid service was used or tested. These tool capabilities do not establish accuracy on these portals. Accessed 11 October 2026.

## Run the collector

Python 3 runs the default collector with its standard library. The optional `curl` transport requires curl. The system curl client verified TLS successfully on four hosts where Python certificate validation failed during the first trial. TLS verification stays enabled. Source: the transport retests recorded in `organisation-route-results.json`, accessed 11 October 2026.

```bash
python3 organisation_tenders.py \
  --url 'https://eproc.rajasthan.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page' \
  --transport curl --max-orgs 0 --timeout 90 \
  --query road --out rajasthan-active.json
```

The default limit is two organisations. `--max-orgs 0` scans every listed organisation, with one second between requests. Use `--organisation` to select a buyer. The output includes the fetched records, local matches, advertised counts, and fetch errors. `complete_active_snapshot` becomes true only when every listed organisation was scanned and every count matched. This verifies the visible active listing at that time. It does not establish archive, award, attachment, or later-session coverage.

## Remaining work

Use this collector as the primary active-list route. Keep CAPTCHA solving as a fallback for Mizoram and protected filters that the active list cannot reproduce. Test a portal-specific OCR model or the image-to-text API before enabling that fallback. Keep historical searches separate from active snapshots.

GeM, Bihar EPS, and the CPPP results-of-tenders page were outside this organisation-route trial. The existing GeM client remains a separate adapter. Their protected searches and coverage gaps remain unverified here. The original 47 entries include two URLs on the same central host. Therefore, these 44 host tests do not mean that 43 of the original 47 search pages are solved.

## Host evidence

Each row is a direct trial result, [Certain], accessed 11 October 2026. Record counts refer only to the sampled organisations.

| Official source | Status | Organisations listed | Records returned |
|---|---|---:|---:|
| [arunachaltenders.gov.in](https://arunachaltenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 2 | 1 |
| [assamtenders.gov.in](https://assamtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 46 | 1 |
| [cgeproc.cgstate.gov.in](https://cgeproc.cgstate.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 1 | 1 |
| [coalindiatenders.nic.in](https://coalindiatenders.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 10 | 150 |
| [ddtenders.gov.in](https://ddtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 2 | 11 |
| [defproc.gov.in](https://defproc.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 10 | 58 |
| [dnhtenders.gov.in](https://dnhtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 1 | 20 |
| [eproc.punjab.gov.in](https://eproc.punjab.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 32 | 39 |
| [eproc.rajasthan.gov.in](https://eproc.rajasthan.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 108 | 7 |
| [eprocure.andamannicobar.gov.in](https://eprocure.andamannicobar.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 18 | 4 |
| [eprocure.goa.gov.in](http://eprocure.goa.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 33 | 3 |
| [eprocure.gov.in](https://eprocure.gov.in/eprocure/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 244 | 1 |
| [eprocurebel.co.in](https://eprocurebel.co.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 7 | 65 |
| [eprocurebhel.co.in](https://eprocurebhel.co.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 1 | 479 |
| [eprocuregsl.nic.in](https://eprocuregsl.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 7 | 65 |
| [eprocurehsl.nic.in](https://eprocurehsl.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 7 | 65 |
| [eprocuremdl.nic.in](https://eprocuremdl.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 7 | 65 |
| [eprocurentpc.nic.in](https://eprocurentpc.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 6 | 5 |
| [etender.up.nic.in](https://etender.up.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 132 | 7 |
| [etenders.chd.nic.in](https://etenders.chd.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 3 | 172 |
| [etenders.gov.in](https://etenders.gov.in/eprocure/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 84 | 3 |
| [etenders.hry.nic.in](https://etenders.hry.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 2 | 828 |
| [etenders.kerala.gov.in](https://etenders.kerala.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 125 | 6 |
| [govtprocurement.delhi.gov.in](https://govtprocurement.delhi.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 21 | 1 |
| [hptenders.gov.in](https://hptenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 48 | 4 |
| [iocletenders.nic.in](https://iocletenders.nic.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 1 | 230 |
| [jharkhandtenders.gov.in](https://jharkhandtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 38 | 10 |
| [jktenders.gov.in](https://jktenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 34 | 141 |
| [mahatenders.gov.in](https://mahatenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 134 | 10 |
| [manipurtenders.gov.in](https://manipurtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 14 | 5 |
| [meghalayatenders.gov.in](https://meghalayatenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 5 | 9 |
| [mizoramtenders.gov.in](https://mizoramtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | challenge_required | 0 | 0 |
| [mptenders.gov.in](http://mptenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 97 | 16 |
| [nagalandtenders.gov.in](https://nagalandtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 3 | 1 |
| [pudutenders.gov.in](https://pudutenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 10 | 2 |
| [sikkimtender.gov.in](https://sikkimtender.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 3 | 1 |
| [tenders.ladakh.gov.in](https://tenders.ladakh.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 6 | 23 |
| [tendersodisha.gov.in](https://tendersodisha.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 67 | 2 |
| [tendersutl.gov.in](https://tendersutl.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 2 | 2 |
| [tntenders.gov.in](https://tntenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 66 | 6 |
| [tripuratenders.gov.in](https://tripuratenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 35 | 2 |
| [uktenders.gov.in](https://uktenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 72 | 8 |
| [wbtenders.gov.in](https://wbtenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 94 | 1 |
| [www.pmgsytenders.gov.in](https://www.pmgsytenders.gov.in/nicgep/app?page=FrontEndTendersByOrganisation&service=page) | records_returned | 16 | 3 |

Validation: three parser regression tests passed. They check mixed-case tender IDs, ignored HTML comments, deduplication, session links, and rejection of non-record tables. The full Arunachal scan checked both directory counts against returned records.
