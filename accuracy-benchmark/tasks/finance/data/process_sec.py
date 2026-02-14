#!/usr/bin/env python3
"""Process SEC EDGAR Financial Statement Data Set into a rich JSON sample.

Downloads the quarterly ZIP archive from SEC EDGAR, extracts the 4 relational
tables (SUB, NUM, TAG, PRE), joins them, and produces fully-resolved financial
statements for selected companies.

Usage:
    python process_sec.py                    # defaults to 2025q4
    python process_sec.py --quarter 2025q3   # specify quarter

Requirements:
    pip install polars requests

Output: sec_edgar_{quarter}.json in the same directory.
"""

import argparse
import io
import json
import tempfile
import zipfile
from pathlib import Path
import requests

import polars as pl

DATA_DIR = Path(__file__).parent
SEC_BASE_URL = "https://www.sec.gov/files/dera/data/financial-statement-data-sets"

# SEC requires a User-Agent header for programmatic access
USER_AGENT = "TeaLeaf-Benchmark/1.0 benchmark@tealeaf-project.dev"


def download_and_extract(quarter: str) -> Path:
    """Download the SEC EDGAR ZIP and extract to a temp directory."""
    url = f"{SEC_BASE_URL}/{quarter}.zip"
    print(f"Downloading {url} ...")

    resp = requests.get(url, headers={"User-Agent": USER_AGENT})
    resp.raise_for_status()
    zip_bytes = resp.content

    print(f"  Downloaded {len(zip_bytes):,} bytes")

    extract_dir = Path(tempfile.mkdtemp(prefix=f"sec_{quarter}_"))
    with zipfile.ZipFile(io.BytesIO(zip_bytes)) as zf:
        zf.extractall(extract_dir)
        print(f"  Extracted {len(zf.namelist())} files to {extract_dir}")

    return extract_dir


def load_tables(raw_dir: Path) -> tuple:
    """Load all four SEC EDGAR tables from a directory."""
    print("Loading SUB...")
    sub = pl.read_csv(
        raw_dir / "sub.txt",
        separator="\t",
        infer_schema_length=0,
        null_values=[""],
    )
    print(f"  SUB: {sub.shape[0]:,} rows x {sub.shape[1]} cols")

    print("Loading TAG...")
    tag = pl.read_csv(
        raw_dir / "tag.txt",
        separator="\t",
        infer_schema_length=0,
        null_values=[""],
    )
    print(f"  TAG: {tag.shape[0]:,} rows x {tag.shape[1]} cols")

    print("Loading NUM...")
    num = pl.read_csv(
        raw_dir / "num.txt",
        separator="\t",
        infer_schema_length=0,
        null_values=[""],
    )
    print(f"  NUM: {num.shape[0]:,} rows x {num.shape[1]} cols")

    print("Loading PRE...")
    pre = pl.read_csv(
        raw_dir / "pre.txt",
        separator="\t",
        infer_schema_length=0,
        null_values=[""],
    )
    print(f"  PRE: {pre.shape[0]:,} rows x {pre.shape[1]} cols")

    return sub, tag, num, pre


def find_companies(sub: pl.DataFrame) -> pl.DataFrame:
    """Find target companies from 10-K annual filings."""
    target_names = [
        "APPLE INC",
        "MICROSOFT CORP",
        "ALPHABET INC.",
        "AMAZON COM INC",
        "NVIDIA CORP",
    ]

    # Filter SUB for annual (10-K) filings, most recent first
    annual_subs = (
        sub
        .filter(pl.col("form").str.contains("10-K"))
        .filter(pl.col("fp") == "FY")
        .sort("period", descending=True)
    )

    # Try to find our targets; fall back to partial match if needed
    matched = annual_subs.filter(
        pl.col("name").str.to_uppercase().is_in([n.upper() for n in target_names])
    )
    if matched.shape[0] == 0:
        print("  Exact matches not found, trying partial match...")
        conditions = pl.lit(False)
        for name in target_names:
            conditions = conditions | pl.col("name").str.to_uppercase().str.contains(name.split()[0])
        matched = annual_subs.filter(conditions)

    # Take one filing per company (most recent)
    matched = matched.unique(subset=["cik"], keep="first")
    print(f"\nMatched {matched.shape[0]} companies:")
    for row in matched.iter_rows(named=True):
        print(f"  {row['name']} | CIK={row['cik']} | form={row['form']} period={row['period']} adsh={row['adsh']}")

    # If we still don't have enough, broaden search to well-known large-cap companies
    if matched.shape[0] < 5:
        extra_names = [
            "VISA", "MASTERCARD", "INTEL", "AMD", "COSTCO", "NIKE",
            "COCA-COLA", "JPMORGAN", "BANK OF AMERICA", "WALMART",
            "PROCTER", "JOHNSON & JOHNSON", "PFIZER", "MERCK",
            "ADOBE", "SALESFORCE", "ORACLE", "CISCO", "QUALCOMM",
        ]
        existing_ciks = set(matched["cik"].to_list())
        for extra in extra_names:
            if matched.shape[0] >= 5:
                break
            hit = annual_subs.filter(
                pl.col("name").str.to_uppercase().str.contains(extra)
                & ~pl.col("cik").is_in(list(existing_ciks))
            ).head(1)
            if hit.shape[0] > 0:
                matched = pl.concat([matched, hit])
                existing_ciks.add(hit["cik"][0])

        print(f"  Expanded to {matched.shape[0]} companies:")
        for row in matched.iter_rows(named=True):
            print(f"  {row['name']} | CIK={row['cik']} | form={row['form']} period={row['period']}")

    return matched


def join_tables(matched: pl.DataFrame, tag: pl.DataFrame,
                num: pl.DataFrame, pre: pl.DataFrame) -> pl.DataFrame:
    """Filter and join NUM, TAG, PRE tables for matched companies."""
    target_adshs = matched["adsh"].to_list()

    print(f"\nFiltering NUM to {len(target_adshs)} filings...")
    num_filtered = num.filter(pl.col("adsh").is_in(target_adshs))
    print(f"  NUM filtered: {num_filtered.shape[0]:,} rows")

    print("Filtering PRE to target filings...")
    pre_filtered = pre.filter(pl.col("adsh").is_in(target_adshs))
    print(f"  PRE filtered: {pre_filtered.shape[0]:,} rows")

    # Cast numeric columns
    num_filtered = num_filtered.with_columns(
        pl.col("value").cast(pl.Float64, strict=False),
        pl.col("qtrs").cast(pl.Int32, strict=False),
    )
    pre_filtered = pre_filtered.with_columns(
        pl.col("report").cast(pl.Int32, strict=False),
        pl.col("line").cast(pl.Int32, strict=False),
    )

    # NUM -> SUB (company info)
    print("\nJoining NUM -> SUB...")
    num_sub = num_filtered.join(
        matched.select("adsh", "cik", "name", "sic", "stprba", "cityba", "form", "period", "fy", "fp", "filed"),
        on="adsh",
        how="left",
    )

    # NUM -> TAG (human-readable label + doc)
    print("Joining NUM -> TAG...")
    num_sub_tag = num_sub.join(
        tag.select("tag", "version", "tlabel", "doc", "iord", "crdr"),
        on=["tag", "version"],
        how="left",
    )

    # NUM + PRE (statement type, presentation order, display label)
    print("Joining with PRE...")
    enriched = num_sub_tag.join(
        pre_filtered.select("adsh", "tag", "version", "report", "line", "stmt", "plabel"),
        on=["adsh", "tag", "version"],
        how="left",
    )

    print(f"  Enriched dataset: {enriched.shape[0]:,} rows x {enriched.shape[1]} cols")
    return enriched


STMT_NAMES = {
    "BS": "Balance Sheet",
    "IS": "Income Statement",
    "CF": "Cash Flows",
    "EQ": "Stockholders Equity",
    "CI": "Comprehensive Income",
    "CP": "Cover Page",
    None: "Other",
}


def build_json(matched: pl.DataFrame, enriched: pl.DataFrame, quarter: str) -> dict:
    """Build structured JSON output per company."""
    target_adshs = matched["adsh"].to_list()
    companies = []

    for adsh in target_adshs:
        company_rows = enriched.filter(pl.col("adsh") == adsh)
        if company_rows.shape[0] == 0:
            continue

        first = company_rows.row(0, named=True)
        company_info = {
            "name": first.get("name"),
            "cik": first.get("cik"),
            "sic": first.get("sic"),
            "state": first.get("stprba"),
            "city": first.get("cityba"),
            "filing": {
                "adsh": adsh,
                "form": first.get("form"),
                "period": first.get("period"),
                "fiscal_year": first.get("fy"),
                "fiscal_period": first.get("fp"),
                "filed": first.get("filed"),
            },
        }

        # Group by statement type
        stmt_groups = {}
        # Filter to USD only, no segments (consolidated), standard tags
        core_rows = company_rows.filter(
            (pl.col("uom") == "USD")
            & (pl.col("segments").is_null() | (pl.col("segments") == ""))
            & (pl.col("coreg").is_null() | (pl.col("coreg") == ""))
        )

        # Use the most recent period for each tag (largest ddate)
        core_rows = core_rows.sort("ddate", descending=True).unique(
            subset=["tag", "stmt"], keep="first"
        )

        for row in core_rows.sort(["stmt", "line"]).iter_rows(named=True):
            stmt_code = row.get("stmt") or "Other"
            stmt_name = STMT_NAMES.get(stmt_code, stmt_code)

            if stmt_name not in stmt_groups:
                stmt_groups[stmt_name] = []

            value = row.get("value")
            if value is None:
                continue

            crdr = row.get("crdr")
            doc = row.get("doc")

            item = {
                "tag": row.get("tag") or "",
                "label": row.get("plabel") or row.get("tlabel") or row.get("tag") or "",
                "value": value,
                "period_end": row.get("ddate") or "",
                "quarters": row.get("qtrs") if row.get("qtrs") is not None else 0,
                "normal_balance": ("credit" if crdr == "C" else "debit") if crdr else "",
                "definition": (doc[:200] + ("..." if len(doc) > 200 else "")) if doc and len(doc) > 10 else "",
            }

            stmt_groups[stmt_name].append(item)

        # Convert to a list of uniformly-shaped statement objects
        statements = [
            {"statement_type": name, "items": items}
            for name, items in stmt_groups.items()
        ]

        company_info["statements"] = statements
        companies.append(company_info)

    # Summary statistics
    total_items = sum(
        len(s["items"]) for co in companies for s in co.get("statements", [])
    )
    print(f"\nBuilt {len(companies)} company filings with {total_items} line items total")

    for co in companies:
        name = co["name"]
        stmts = {s["statement_type"]: len(s["items"]) for s in co.get("statements", [])}
        print(f"  {name}: {stmts}")

    return {
        "source": f"SEC EDGAR Financial Statement Data Sets ({quarter.upper()})",
        "license": "US Government Public Domain",
        "url": "https://www.sec.gov/dera/data/financial-statement-data-sets",
        "description": "XBRL financial statement data from annual 10-K filings, joined across SUB/NUM/TAG/PRE tables",
        "companies": companies,
    }


def main():
    parser = argparse.ArgumentParser(
        description="Download and process SEC EDGAR Financial Statement Data Set"
    )
    parser.add_argument(
        "--quarter", default="2025q4",
        help="Quarter to download, e.g. 2025q4, 2025q3 (default: 2025q4)"
    )
    args = parser.parse_args()

    quarter = args.quarter

    # Step 1: Download and extract
    raw_dir = download_and_extract(quarter)

    try:
        # Step 2: Load tables
        sub, tag, num, pre = load_tables(raw_dir)

        # Step 3: Find target companies
        matched = find_companies(sub)

        if matched.shape[0] == 0:
            print("Error: No matching companies found in this quarter's data.")
            return

        # Step 4: Join tables
        enriched = join_tables(matched, tag, num, pre)

        # Step 5: Build JSON
        output = build_json(matched, enriched, quarter)

        # Step 6: Write output
        # Insert underscore before q-suffix: "2025q4" -> "2025_q4"
        file_quarter = quarter[:4] + "_" + quarter[4:]
        out_path = DATA_DIR / f"sec_edgar_{file_quarter}.json"
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(output, f, indent=2, ensure_ascii=False)

        size = out_path.stat().st_size
        print(f"\nWritten: {out_path}")
        print(f"Size: {size:,} bytes ({size/1024:.1f} KB)")

    finally:
        # Clean up temp directory
        import shutil
        shutil.rmtree(raw_dir, ignore_errors=True)
        print(f"Cleaned up temp directory: {raw_dir}")


if __name__ == "__main__":
    main()
