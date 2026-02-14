#!/usr/bin/env python3
"""Process CDC PLACES county-level health data into a benchmark JSON file.

Downloads health indicator data from the CDC PLACES SODA API (data.cdc.gov),
loads it into a Polars DataFrame, filters to a representative subset of counties
for a single state, and produces a hierarchically structured JSON file: counties
contain category groups, each containing measures with nested confidence intervals.

Usage:
    python process_places.py                          # defaults to WA state
    python process_places.py --state CA               # different state
    python process_places.py --state WA --limit 10    # limit county count

Requirements:
    pip install polars requests

Output: places_{state}_health_{year}.json in the same directory.
"""

import argparse
import json
from pathlib import Path
from urllib.parse import urlencode
from urllib.request import urlopen, Request

import polars as pl

DATA_DIR = Path(__file__).parent

SODA_BASE = "https://data.cdc.gov/resource/swc5-untb.json"

# Dataset metadata
DATASET_ID = "swc5-untb"
DATASET_URL = f"https://data.cdc.gov/500-Cities-Places/PLACES-Local-Data-for-Better-Health-County-Data-20/{DATASET_ID}"

# Category definitions (stable across releases)
CATEGORIES = [
    {"id": "HLTHOUT", "name": "Health Outcomes", "description": "Chronic disease and condition prevalence"},
    {"id": "HLTHSTAT", "name": "Health Status", "description": "Self-reported health and distress measures"},
    {"id": "PREVENT", "name": "Prevention", "description": "Preventive service utilization"},
    {"id": "RISKBEH", "name": "Health Risk Behaviors", "description": "Behavioral risk factors"},
    {"id": "DISABLT", "name": "Disability", "description": "Disability prevalence measures"},
    {"id": "HRSN", "name": "Health-Related Social Needs", "description": "Social determinants of health"},
]

# Default county selections per state (mix of urban, suburban, rural)
STATE_COUNTIES = {
    "WA": ["King", "Pierce", "Spokane", "Clark", "Yakima", "Whatcom", "Benton", "Island", "Ferry", "Garfield"],
    "CA": ["Los Angeles", "San Diego", "San Francisco", "Sacramento", "Fresno", "Alameda", "Riverside", "Santa Barbara", "Humboldt", "Inyo"],
    "OR": ["Multnomah", "Lane", "Marion", "Deschutes", "Jackson", "Clackamas", "Benton", "Umatilla", "Josephine", "Wheeler"],
}

# Columns to fetch from the SODA API
SODA_COLUMNS = [
    "year", "stateabbr", "statedesc", "locationname",
    "category", "measure", "data_value_unit", "data_value_type",
    "data_value", "low_confidence_limit", "high_confidence_limit",
    "totalpopulation", "totalpop18plus",
    "locationid", "categoryid", "measureid", "datavaluetypeid",
    "short_question_text",
]


def fetch_records(state: str, limit: int = 5000) -> pl.DataFrame:
    """Fetch PLACES records for a state from the SODA API into a DataFrame."""
    params = {
        "stateabbr": state,
        "$limit": str(limit),
        "$select": ",".join(SODA_COLUMNS),
    }
    url = f"{SODA_BASE}?{urlencode(params)}"
    print(f"Fetching PLACES data for {state}...")
    print(f"  URL: {url[:120]}...")

    req = Request(url, headers={"Accept": "application/json"})
    with urlopen(req) as resp:
        raw = json.loads(resp.read().decode("utf-8"))

    print(f"  Got {len(raw)} records")

    # Load into Polars and cast numeric columns
    df = pl.DataFrame(raw)
    df = df.with_columns(
        pl.col("data_value").cast(pl.Float64, strict=False),
        pl.col("low_confidence_limit").cast(pl.Float64, strict=False),
        pl.col("high_confidence_limit").cast(pl.Float64, strict=False),
        pl.col("totalpopulation").cast(pl.Int64, strict=False),
        pl.col("totalpop18plus").cast(pl.Int64, strict=False),
    )

    return df


def select_counties(df: pl.DataFrame, n: int) -> list[str]:
    """Auto-select the N most populous counties from the dataset."""
    top = (
        df
        .group_by("locationname")
        .agg(pl.col("totalpopulation").max().alias("pop"))
        .sort("pop", descending=True)
        .head(n)
    )
    counties = top["locationname"].to_list()
    print(f"  Auto-selected {len(counties)} most populous counties: {', '.join(counties)}")
    return counties


def filter_and_build(df: pl.DataFrame, state: str, counties: list[str]) -> dict:
    """Filter to target counties and crude prevalence, build nested output.

    Output structure:
        counties[] → { demographics, health_data: { category_id: measures[] } }
        Each measure has a nested confidence_interval object.
    """
    # Filter to target counties + crude prevalence only
    filtered = df.filter(
        pl.col("locationname").is_in(counties)
        & (pl.col("datavaluetypeid") == "CrdPrv")
    )
    print(f"Filtered to {filtered.shape[0]} records ({len(counties)} counties, crude prevalence)")

    if filtered.shape[0] == 0:
        raise ValueError(f"No records matched for state={state}, counties={counties}")

    # Drop rows with null numeric values
    filtered = filtered.drop_nulls(subset=["data_value", "low_confidence_limit", "high_confidence_limit"])
    print(f"  After dropping nulls: {filtered.shape[0]} records")

    # Detect year and state name from data
    year = int(filtered["year"][0])
    state_name = filtered["statedesc"][0]

    # Count unique measures
    measure_ids = filtered["measureid"].unique().sort()
    measure_count = measure_ids.len()

    # Category ID ordering for consistent output
    cat_order = {cat["id"]: i for i, cat in enumerate(CATEGORIES)}

    # Build nested county objects
    county_order = {name: i for i, name in enumerate(counties)}
    county_list = []
    total_measures = 0

    for county_name in counties:
        county_rows = filtered.filter(pl.col("locationname") == county_name)
        if county_rows.shape[0] == 0:
            continue

        first = county_rows.row(0, named=True)

        # Group measures by category
        health_data = {}
        for cat in CATEGORIES:
            cat_id = cat["id"]
            cat_rows = county_rows.filter(pl.col("categoryid") == cat_id).sort("measureid")
            if cat_rows.shape[0] == 0:
                continue

            measures = []
            for row in cat_rows.iter_rows(named=True):
                measures.append({
                    "measure_id": row["measureid"],
                    "short_name": row["short_question_text"],
                    "measure": row["measure"],
                    "value": row["data_value"],
                    "confidence_interval": {
                        "low": row["low_confidence_limit"],
                        "high": row["high_confidence_limit"],
                    },
                })
                total_measures += 1

            health_data[cat_id] = measures

        county_list.append({
            "name": county_name,
            "fips": first["locationid"],
            "population": first["totalpopulation"],
            "adult_population": first["totalpop18plus"],
            "health_data": health_data,
        })

    print(f"Built {len(county_list)} counties with {total_measures} total measures")

    return {
        "source": "CDC PLACES: Local Data for Better Health, County Data 2025 Release",
        "license": "US Government Public Domain",
        "url": DATASET_URL,
        "description": (
            f"County-level crude prevalence estimates for {measure_count} health measures "
            f"from BRFSS. {len(county_list)} {state_name} counties representing urban, "
            f"suburban, and rural populations."
        ),
        "year": year,
        "state": state_name,
        "data_source": "Behavioral Risk Factor Surveillance System (BRFSS)",
        "measure_count": measure_count,
        "record_count": total_measures,
        "categories": CATEGORIES,
        "counties": county_list,
    }


def main():
    parser = argparse.ArgumentParser(
        description="Download and process CDC PLACES county health indicator data"
    )
    parser.add_argument(
        "--state", default="WA",
        help="Two-letter state abbreviation (default: WA)"
    )
    parser.add_argument(
        "--limit", type=int, default=10,
        help="Number of counties to include (default: 10)"
    )
    parser.add_argument(
        "--counties", nargs="*",
        help="Explicit county names (overrides --limit and default selection)"
    )
    args = parser.parse_args()

    state = args.state.upper()

    # Step 1: Fetch all records for the state
    df = fetch_records(state)

    # Step 2: Determine county list
    if args.counties:
        counties = args.counties
    elif state in STATE_COUNTIES:
        counties = STATE_COUNTIES[state][:args.limit]
    else:
        counties = select_counties(df, args.limit)

    # Step 3: Filter and build structured output
    output = filter_and_build(df, state, counties)

    # Step 4: Write output
    year = output["year"]
    out_path = DATA_DIR / f"places_{state.lower()}_health_{year}.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    size = out_path.stat().st_size
    print(f"\nWritten: {out_path}")
    print(f"Size: {size:,} bytes ({size / 1024:.1f} KB)")
    print(f"Counties: {len(output['counties'])}")
    print(f"Measures: {output['measure_count']}")
    print(f"Records: {output['record_count']}")
    cats = len(output['counties'][0]['health_data']) if output['counties'] else 0
    print(f"Categories per county: {cats}")


if __name__ == "__main__":
    main()
