# Backend gate check, 11 Oct 2026

Question: on the 47 challenge portals, is the captcha or OTP enforced by the backend, or is it only on the form while rows sit in the open? Method: public GETs only. We followed each site's own links and tried the known GePNIC page names. No captcha, OTP, or login was submitted, and no search POST was sent.

## Verdict

The captcha is a real gate on search everywhere we looked. It is decoration on the by-date listing of six hosts: those pages already contain a full tender table for the day, captcha box and all.

### Rows open without solving anything

| Host | Rows on the default by-date page | Note |
| --- | ---: | --- |
| eproc.rajasthan.gov.in | 8 | `FrontEndListTendersbyDate`, full titles and org chain |
| hptenders.gov.in | 4 | same page name |
| defproc.gov.in | 3 | Military Engineer Services works |
| etenders.hry.nic.in | 1 | ICDS food procurement visible |
| coalindiatenders.nic.in | 1 | Mahanadi Coalfields civil works |
| jktenders.gov.in | 1 | serves unverified TLS certificate |

These pages list tenders closing on one day. Paging to another day needs a session token from a click. The captcha on the page belongs to the search form, not the listing.

### Enforced gates, checked and closed

- Bihar `EPSV2Web`: Angular app with bcrypt and Google reCAPTCHA in the home bundle. No tender rows on any public page we found.
- Telangana and Andhra Pradesh eProc-Suite: home loads a `jCryption` RSA handshake script before any form works. Direct links die without a session cookie.
- `tender.nprocure.com` (Gujarat): captcha plus an OTP on the same page.
- IREPS: works search posts to `guestLogin.do`, mobile OTP.
- Every other GePNIC host: the listing pages print "Provide Captcha and click on Search button" and serve no rows. Defence, PMGSY, Coal India group hosts, BHEL, NTPC, Indian Oil, BEL, HSL, MDL, GSL are the same build.

### The open copy of the closed portals

All state GePNIC notices are mirrored on the CPPP lists, which need no captcha at all: 54,759 state rows and 31,070 central rows, plus 575 high-value and 11 closing-today. The six by-date pages above add fresh rows a few hours earlier than the mirror. Nothing else on the 47 adds coverage.

### Also checked

- CPPP state detail links: with the list page as referer the "Invalid Url" error goes away, but the tender fields are not in the first HTML. Titles only, as before.
- GeM BidPlus: the csrf token in the HTML is a normal session token, not a captcha. The data endpoint answers publicly; 46,049 ongoing and 5,744,348 awarded bids counted.

## Bottom line

41 of 47 keep their data behind a working challenge. 6 leak the day's table on the by-date page. The practical open backend for all of them remains the CPPP aggregate, which is where gem-probe should read next.