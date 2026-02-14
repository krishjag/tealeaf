#!/usr/bin/env python3
"""Fetch property data from NYC Open Data PLUTO (Primary Land Use Tax Lot Output).

Downloads tax lot records from NYC's MapPLUTO dataset via the Socrata Open Data
API with minimal processing — the native field names and structure are preserved.
Numeric strings (a Socrata serialization quirk) are converted to actual numbers.

Usage:
    python process_nyc_pluto.py                                  # default: 30 Manhattan properties
    python process_nyc_pluto.py --borough MN --limit 30          # custom borough and limit
    python process_nyc_pluto.py --borough BK --landuse 01 --limit 25

Requirements:
    pip install requests
"""

import argparse
import json
import time
from pathlib import Path

import requests

DATA_DIR = Path(__file__).parent

# NYC Open Data PLUTO dataset (25v3.1)
PLUTO_API = "https://data.cityofnewyork.us/resource/64uk-42ks.json"


def fetch_properties(
    borough: str = "MN",
    landuse: str | None = None,
    limit: int = 30,
) -> list[dict]:
    """Fetch tax lot records from NYC PLUTO via Socrata API."""

    # Build SoQL query for diverse property mix
    where_clauses = [f"borough='{borough}'"]
    if landuse:
        where_clauses.append(f"landuse='{landuse}'")
    # Ensure meaningful records (built, assessed, with buildings)
    where_clauses.append("yearbuilt > '1800'")
    where_clauses.append("assesstot > '0'")
    where_clauses.append("bldgarea > '0'")

    params = {
        "$limit": limit,
        "$where": " AND ".join(where_clauses),
        "$order": "assesstot DESC",  # Most valuable first for interesting analysis
    }

    headers = {
        "User-Agent": "TeaLeaf-Benchmark/1.0 benchmark@tealeaf-project.dev",
        "Accept": "application/json",
    }

    print(f"  Querying: {params['$where']}")
    resp = requests.get(PLUTO_API, params=params, headers=headers, timeout=60)
    resp.raise_for_status()
    properties = resp.json()

    print(f"  Fetched {len(properties)} tax lots")
    return properties


def coerce_numeric(value: str) -> int | float | str:
    """Convert numeric strings to actual numbers (Socrata serialization fix)."""
    if not isinstance(value, str):
        return value
    # Try integer first
    try:
        # Check if it looks like an integer (no decimal point or .000...)
        if "." not in value:
            return int(value)
        # Check if all decimals are zero (e.g., "31260.00000")
        float_val = float(value)
        if float_val == int(float_val) and abs(float_val) < 1e15:
            return int(float_val)
        return float_val
    except (ValueError, OverflowError):
        return value


def clean_record(record: dict) -> dict:
    """Clean a single PLUTO record: convert numeric strings, strip nulls/empties."""
    cleaned = {}
    for key, value in record.items():
        if value is None or value == "":
            continue
        if isinstance(value, str):
            cleaned[key] = coerce_numeric(value)
        else:
            cleaned[key] = value
    return cleaned


def summarize(properties: list[dict]) -> dict:
    """Build summary statistics from PLUTO records."""
    land_uses = {}
    bldg_classes = {}
    boroughs = {}
    year_min = 9999
    year_max = 0
    total_assessed = 0

    for prop in properties:
        # Land use
        lu = prop.get("landuse", "Unknown")
        land_uses[lu] = land_uses.get(lu, 0) + 1

        # Building class
        bc = prop.get("bldgclass", "Unknown")
        bldg_classes[bc] = bldg_classes.get(bc, 0) + 1

        # Borough
        b = prop.get("borough", "Unknown")
        boroughs[b] = boroughs.get(b, 0) + 1

        # Year built
        yb = prop.get("yearbuilt", 0)
        if isinstance(yb, (int, float)) and yb > 1800:
            year_min = min(year_min, int(yb))
            year_max = max(year_max, int(yb))

        # Assessment
        at = prop.get("assesstot", 0)
        if isinstance(at, (int, float)):
            total_assessed += at

    return {
        "totalProperties": len(properties),
        "boroughDistribution": dict(sorted(boroughs.items())),
        "landUseDistribution": dict(sorted(land_uses.items(), key=lambda x: -x[1])[:10]),
        "topBuildingClasses": dict(sorted(bldg_classes.items(), key=lambda x: -x[1])[:10]),
        "yearBuiltRange": {"earliest": year_min, "latest": year_max},
        "totalAssessedValue": total_assessed,
    }


LAND_USE_LABELS = {
    1: "One & Two Family Buildings",
    2: "Multi-Family Walk-Up",
    3: "Multi-Family Elevator",
    4: "Mixed Residential & Commercial",
    5: "Commercial & Office",
    6: "Industrial & Manufacturing",
    7: "Transportation & Utility",
    8: "Public Facilities & Institutions",
    9: "Open Space & Outdoor Recreation",
    10: "Parking Facilities",
    11: "Vacant Land",
}


def main():
    parser = argparse.ArgumentParser(
        description="Fetch NYC PLUTO property data for benchmark"
    )
    parser.add_argument(
        "--borough", default="MN",
        help="Borough code: MN, BX, BK, QN, SI (default: MN)"
    )
    parser.add_argument(
        "--landuse", default=None,
        help="Land use filter (01-11), omit for all types"
    )
    parser.add_argument(
        "--limit", type=int, default=30,
        help="Maximum number of properties to fetch (default: 30)"
    )
    args = parser.parse_args()

    borough_names = {"MN": "Manhattan", "BX": "Bronx", "BK": "Brooklyn", "QN": "Queens", "SI": "Staten Island"}
    borough_name = borough_names.get(args.borough, args.borough)

    print(f"Fetching property data from NYC PLUTO...")
    print(f"  Borough: {borough_name} ({args.borough})")
    print(f"  Land use: {args.landuse or '(all types)'}")
    print(f"  Limit: {args.limit}")

    # Fetch raw records
    raw_props = fetch_properties(args.borough, args.landuse, args.limit)

    if not raw_props:
        print("No properties found matching criteria.")
        return

    # Minimal cleanup: convert Socrata numeric strings to numbers, strip empties
    print("Cleaning records (fixing Socrata numeric strings, stripping empties)...")
    properties = [clean_record(prop) for prop in raw_props]

    # Build summary
    summary = summarize(properties)
    print(f"  Land uses: {summary['landUseDistribution']}")
    print(f"  Year range: {summary['yearBuiltRange']}")
    print(f"  Total assessed value: ${summary['totalAssessedValue']:,.0f}")

    # Build output
    output = {
        "source": "NYC Open Data — PLUTO (Primary Land Use Tax Lot Output)",
        "license": "Public Domain (NYC Open Data Terms of Use)",
        "url": "https://data.cityofnewyork.us/City-Government/Primary-Land-Use-Tax-Lot-Output-PLUTO-/64uk-42ks",
        "api": "Socrata Open Data API (SODA)",
        "description": (
            f"{len(properties)} tax lot records from NYC PLUTO for {borough_name}. "
            f"Each record has ~90 fields covering geographic identifiers, zoning, "
            f"building dimensions, area breakdowns, assessed values, FAR, "
            f"ownership, and administrative districts. "
            f"Socrata numeric-string serialization corrected to native types."
        ),
        "query": {
            "borough": args.borough,
            "boroughName": borough_name,
            "landUse": args.landuse,
            "orderBy": "assesstot DESC",
            "fetchedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        },
        "landUseCodeReference": {str(k): v for k, v in LAND_USE_LABELS.items()},
        "summary": summary,
        "properties": properties,
    }

    # Write output
    slug = f"pluto_{borough_name.lower()}"
    if args.landuse:
        lu_label = LAND_USE_LABELS.get(int(args.landuse), args.landuse).lower().replace(" ", "_")
        slug += f"_{lu_label}"
    out_path = DATA_DIR / f"{slug}.json"

    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(output, fh, indent=2, ensure_ascii=False)

    size = out_path.stat().st_size
    print(f"\nWritten: {out_path}")
    print(f"Size: {size:,} bytes ({size / 1024:.1f} KB)")
    print(f"Properties: {len(properties)}")
    print(f"Fields per record: ~{len(properties[0]) if properties else 0}")


if __name__ == "__main__":
    main()
