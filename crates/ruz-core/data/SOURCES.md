# Data sources

Ruz's calendar data is written by its authors from public sources. Dates are
facts; the selection, grouping and wording of occasions are our own. Nothing is
copied from other calendar apps.

| Key | Used for | Source |
|---|---|---|
| `iran-official-holidays` | Official days off | Iran's law on official holidays, as listed in the official calendar of the Calendar Center, Institute of Geophysics, University of Tehran |
| `iran-official-calendar` | Other official occasions, appendix and professions days | The same official calendar. Dates checked against the 1404 and 1405 SH calendars as shown on time.ir |
| `un-international-days` | International days | United Nations and WHO observances listed in the official calendar |
| `iranian-festivals` | Historical Iranian festivals | Their customary Solar Hijri dates, as listed in the official calendar and time.ir |
| `shia-occasions` | Further Shia occasions | Their customary Lunar Hijri dates |

`hijri_iran.toml` holds Iran's official Lunar Hijri month starts, read from the
day-by-day Hijri dates time.ir shows for each Solar Hijri year (time.ir follows
the University of Tehran calendar). Each year records its source and the date
it was read.

`tests/fixtures/official_days_off.toml` lists each year's official days off from
the same calendar, so the tests catch data or Hijri-table mistakes.

`cities.toml` lists city-centre coordinates (to about 0.01°) and IANA time zones,
compiled by the Ruz authors. Prayer times computed for them are checked against
Iran's official times for Tehran, Mashhad and Bandar Abbas on 23 Mehr 1405, as
shown on time.ir.
