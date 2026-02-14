# NYC PLUTO — Primary Land Use Tax Lot Output

## Source

**NYC Open Data — PLUTO (Primary Land Use Tax Lot Output)**
https://data.cityofnewyork.us/City-Government/Primary-Land-Use-Tax-Lot-Output-PLUTO-/64uk-42ks

## License

Public Domain (NYC Open Data Terms of Use)

## How This Data Was Produced

Tax lot records are fetched from the NYC PLUTO dataset via the Socrata Open
Data API (SODA) with **minimal processing** — the native ~90 field names and
flat structure are preserved. The only transformation is correcting Socrata's
numeric-string serialization (e.g., `"31260.00000"` → `31260`) back to native
numeric types, plus stripping null/empty fields.

### Steps

1. Query the Socrata API at `https://data.cityofnewyork.us/resource/64uk-42ks.json`
   with borough filter, building/assessment constraints, ordered by assessed value
2. Convert Socrata numeric-string serialization to native JSON numbers
3. Strip null values and empty strings
4. Wrap in metadata envelope with summary statistics and land use code reference

### Reproducing

```bash
# Install dependencies
pip install requests

# Fetch default dataset (30 Manhattan properties, highest assessed value)
python process_nyc_pluto.py

# Custom borough and limit
python process_nyc_pluto.py --borough BK --limit 25

# Filter by land use (01=residential, 05=commercial)
python process_nyc_pluto.py --borough MN --landuse 05 --limit 30
```

### Output Schema

Each record is flat with ~76-90 fields covering geographic identifiers, zoning,
building dimensions, area breakdowns, assessed values, FAR, and districts:

```json
{
  "source": "NYC Open Data — PLUTO (Primary Land Use Tax Lot Output)",
  "license": "Public Domain (NYC Open Data Terms of Use)",
  "url": "https://data.cityofnewyork.us/...",
  "api": "Socrata Open Data API (SODA)",
  "description": "30 tax lot records from NYC PLUTO for Manhattan...",
  "query": { "borough", "boroughName", "landUse", "orderBy", "fetchedAt" },
  "landUseCodeReference": {
    "1": "One & Two Family Buildings",
    "2": "Multi-Family Walk-Up",
    "3": "Multi-Family Elevator",
    "4": "Mixed Residential & Commercial",
    "5": "Commercial & Office",
    "...": "..."
  },
  "summary": {
    "totalProperties": 30,
    "boroughDistribution": { "MN": 30 },
    "landUseDistribution": { "5": 23, "8": 3, "4": 3, "3": 1 },
    "topBuildingClasses": { "O4": 15, "O5": 4, ... },
    "yearBuiltRange": { "earliest": 1930, "latest": 2019 },
    "totalAssessedValue": 18262649478
  },
  "properties": [
    {
      "borough": "MN",
      "block": 16,
      "lot": 1,
      "bbl": 1000160001,
      "address": "185 GREENWICH STREET",
      "zipcode": 10007,
      "zonedist1": "C6-4",
      "bldgclass": "O4",
      "landuse": 5,
      "ownername": "THE PORT AUTHORITY OF NY & NJ",
      "lotarea": 59665,
      "bldgarea": 3501274,
      "numfloors": 104,
      "unitsres": 0,
      "unitstotal": 0,
      "yearbuilt": 2009,
      "assessland": 671409000,
      "assesstot": 2613006000,
      "builtfar": 58.68,
      "residfar": 0,
      "commfar": 15,
      "latitude": 40.7130062,
      "longitude": -74.0133883,
      "...": "~76 fields total"
    }
  ]
}
```

This wide flat structure (76 fields repeated across 30 records) produces a single
broad `@struct`/`@table` schema where TL eliminates all 2,280 key-name repetitions
across the dataset.
