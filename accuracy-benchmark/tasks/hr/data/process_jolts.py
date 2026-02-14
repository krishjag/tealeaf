#!/usr/bin/env python3
"""Process BLS JOLTS time series data into a benchmark JSON file.

Downloads the Job Openings and Labor Turnover Survey flat files from
download.bls.gov, loads them into Polars DataFrames, joins with lookup
tables, and produces a hierarchically structured JSON file with industries
containing nested monthly data and per-element level/rate pairs.

Usage:
    python process_jolts.py                        # defaults to 2025
    python process_jolts.py --year 2024            # different year
    python process_jolts.py --year 2025 --limit 12 # limit industries

Requirements:
    pip install polars requests

Output: jolts_{year}.json in the same directory.
"""

import argparse
import io
import json
from pathlib import Path

import polars as pl
import requests

DATA_DIR = Path(__file__).parent

BLS_BASE = "https://download.bls.gov/pub/time.series/jt"

# BLS requires a User-Agent header for programmatic access
USER_AGENT = "TeaLeaf-Benchmark/1.0 benchmark@tealeaf-project.dev"

# Data elements in JOLTS (order matters — used for nesting)
ELEMENT_CODES = ["JO", "HI", "TS", "QU", "LD", "OS"]

ELEMENT_META = {
    "JO": {"name": "Job Openings", "description": "Unfilled positions on the last business day of the month"},
    "HI": {"name": "Hires", "description": "Workers added to payroll during the month"},
    "TS": {"name": "Total Separations", "description": "Workers who left or were removed from payroll during the month"},
    "QU": {"name": "Quits", "description": "Workers who left voluntarily (excludes retirements and transfers)"},
    "LD": {"name": "Layoffs and Discharges", "description": "Involuntary separations initiated by the employer"},
    "OS": {"name": "Other Separations", "description": "Retirements, transfers, disability, and deaths"},
}

# JSON field names for each element code (snake_case for readability)
ELEMENT_KEYS = {
    "JO": "job_openings",
    "HI": "hires",
    "TS": "total_separations",
    "QU": "quits",
    "LD": "layoffs_discharges",
    "OS": "other_separations",
}

MONTH_NAMES = [
    "", "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
]

# Target industries — a representative cross-section of the NAICS hierarchy
# Level 0 = total, level 1 = supersector, level 2 = sector
TARGET_INDUSTRIES = [
    "000000",  # Total nonfarm
    "100000",  # Total private
    "230000",  # Construction
    "300000",  # Manufacturing
    "440000",  # Retail trade
    "510000",  # Information
    "510099",  # Financial activities
    "540099",  # Professional and business services
    "620000",  # Health care and social assistance
    "700000",  # Leisure and hospitality
    "720000",  # Accommodation and food services
    "900000",  # Government
]


def fetch_file(name: str) -> str:
    """Fetch a JOLTS flat file from BLS."""
    url = f"{BLS_BASE}/{name}"
    print(f"  Fetching {name}...")
    resp = requests.get(url, headers={"User-Agent": USER_AGENT})
    resp.raise_for_status()
    return resp.text


def load_lookup(name: str) -> pl.DataFrame:
    """Load a BLS lookup table into a Polars DataFrame."""
    text = fetch_file(name)
    df = pl.read_csv(io.StringIO(text), separator="\t", infer_schema_length=0, null_values=[""])
    # Strip whitespace from column names and string values
    df = df.rename({c: c.strip() for c in df.columns})
    for c in df.columns:
        if df[c].dtype == pl.String or df[c].dtype == pl.Utf8:
            df = df.with_columns(pl.col(c).str.strip_chars())
    return df


def load_data(year: int) -> pl.DataFrame:
    """Load the Current data file and filter to the target year."""
    text = fetch_file("jt.data.0.Current")
    df = pl.read_csv(io.StringIO(text), separator="\t", infer_schema_length=0, null_values=[""])
    df = df.rename({c: c.strip() for c in df.columns})
    for c in df.columns:
        if df[c].dtype == pl.String or df[c].dtype == pl.Utf8:
            df = df.with_columns(pl.col(c).str.strip_chars())

    # Cast types
    df = df.with_columns(
        pl.col("year").cast(pl.Int32),
        pl.col("value").cast(pl.Float64, strict=False),
    )

    print(f"  Loaded {df.shape[0]:,} total data rows")

    # Filter to target year, exclude annual (M13)
    df = df.filter(
        (pl.col("year") == year) & (pl.col("period") != "M13")
    )
    print(f"  Filtered to year {year}: {df.shape[0]:,} rows")

    return df


def build_json(
    data: pl.DataFrame,
    series: pl.DataFrame,
    industry_df: pl.DataFrame,
    target_industries: list[str],
    year: int,
) -> dict:
    """Join data with series/industry lookups and build nested output.

    Output structure:
        industries[] → monthly_data[] → {month, month_name, job_openings: {level, rate}, ...}
    """

    # Filter series to: national, seasonally adjusted, all size classes, target industries
    target_series = series.filter(
        (pl.col("state_code") == "00")
        & (pl.col("seasonal") == "S")
        & (pl.col("sizeclass_code") == "00")
        & (pl.col("industry_code").is_in(target_industries))
        & (pl.col("dataelement_code").is_in(ELEMENT_CODES))
    )
    print(f"\nTarget series: {target_series.shape[0]}")

    # Join data with series metadata
    joined = data.join(
        target_series.select("series_id", "industry_code", "dataelement_code", "ratelevel_code"),
        on="series_id",
        how="inner",
    )
    print(f"Joined observations: {joined.shape[0]:,}")

    # Join with industry names
    joined = joined.join(
        industry_df.select(
            pl.col("industry_code"),
            pl.col("industry_text").alias("industry"),
            pl.col("display_level"),
        ),
        on="industry_code",
        how="left",
    )

    # Extract month number from period code (M01 -> 1)
    joined = joined.with_columns(
        pl.col("period").str.slice(1).cast(pl.Int32).alias("month"),
    )

    # Pivot ratelevel into level/rate columns
    pivoted = (
        joined
        .select("industry_code", "industry", "display_level", "dataelement_code", "month", "ratelevel_code", "value")
        .pivot(on="ratelevel_code", index=["industry_code", "industry", "display_level", "dataelement_code", "month"], values="value")
        .rename({"L": "level", "R": "rate"})
    )

    # Build nested structure: industry → monthly_data → element metrics
    industry_order = {code: i for i, code in enumerate(target_industries)}
    industries = []
    total_months = 0

    for code in target_industries:
        ind_data = pivoted.filter(pl.col("industry_code") == code)
        if ind_data.shape[0] == 0:
            continue

        first = ind_data.row(0, named=True)
        name = first["industry"]
        hierarchy_level = first["display_level"]

        # Build monthly_data array
        monthly_data = []
        for month in range(1, 13):
            month_rows = ind_data.filter(pl.col("month") == month)
            if month_rows.shape[0] == 0:
                continue

            # Build element dict keyed by readable field name
            month_obj = {
                "month": month,
                "month_name": MONTH_NAMES[month],
            }
            for elem_code in ELEMENT_CODES:
                elem_row = month_rows.filter(pl.col("dataelement_code") == elem_code)
                if elem_row.shape[0] > 0:
                    row = elem_row.row(0, named=True)
                    month_obj[ELEMENT_KEYS[elem_code]] = {
                        "level": row["level"],
                        "rate": row["rate"],
                    }

            monthly_data.append(month_obj)

        total_months += len(monthly_data)
        industries.append({
            "code": code,
            "name": name,
            "hierarchy_level": hierarchy_level,
            "monthly_data": monthly_data,
        })

    print(f"Built {len(industries)} industries with {total_months} monthly records")
    print(f"  Each month has 6 elements × 2 values (level + rate) = 12 numeric values")

    # Data elements as reference
    data_elements = [
        {"id": code, "key": ELEMENT_KEYS[code], **ELEMENT_META[code]}
        for code in ELEMENT_CODES
    ]

    return {
        "source": "BLS Job Openings and Labor Turnover Survey (JOLTS)",
        "license": "US Government Public Domain",
        "url": "https://www.bls.gov/jts/",
        "description": (
            f"Monthly seasonally adjusted job openings, hires, and separations data "
            f"for {len(industries)} industries, {year}. Levels in thousands; "
            f"rates as percent of employment."
        ),
        "year": year,
        "seasonal_adjustment": "Seasonally Adjusted",
        "unit_level": "Thousands",
        "unit_rate": "Percent",
        "data_elements": data_elements,
        "industries": industries,
    }


def main():
    parser = argparse.ArgumentParser(
        description="Download and process BLS JOLTS labor turnover data"
    )
    parser.add_argument(
        "--year", type=int, default=2025,
        help="Data year to extract (default: 2025)"
    )
    parser.add_argument(
        "--limit", type=int, default=12,
        help="Number of industries to include (default: 12)"
    )
    args = parser.parse_args()

    year = args.year
    industries = TARGET_INDUSTRIES[:args.limit]

    print(f"Processing JOLTS data for {year}, {len(industries)} industries\n")

    # Step 1: Load lookup tables
    print("Loading lookup tables...")
    industry_df = load_lookup("jt.industry")
    series = load_lookup("jt.series")
    print(f"  Industries: {industry_df.shape[0]}, Series: {series.shape[0]}")

    # Step 2: Load data
    print("\nLoading data...")
    data = load_data(year)

    # Step 3: Build output
    output = build_json(data, series, industry_df, industries, year)

    # Step 4: Write output
    out_path = DATA_DIR / f"jolts_{year}.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    size = out_path.stat().st_size
    print(f"\nWritten: {out_path}")
    print(f"Size: {size:,} bytes ({size / 1024:.1f} KB)")
    print(f"Industries: {len(output['industries'])}")
    months = sum(len(ind['monthly_data']) for ind in output['industries'])
    print(f"Monthly records: {months}")
    print(f"Numeric values: {months * 12}")


if __name__ == "__main__":
    main()
