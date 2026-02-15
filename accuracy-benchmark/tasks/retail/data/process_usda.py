#!/usr/bin/env python3
"""Fetch branded food product data from the USDA FoodData Central API.

Downloads branded food product records with minimal processing — the USDA
response structure is preserved (foods → foodNutrients with 14 fields per
nutrient entry), and we only strip the API pagination envelope, empty arrays,
and null values.

Usage:
    python process_usda.py                                          # default: breakfast cereal, 25 items
    python process_usda.py --query "yogurt" --limit 30              # custom query
    python process_usda.py --query "breakfast cereal" --limit 25

Requirements:
    pip install requests
"""

import argparse
import json
import os
import time
from pathlib import Path

import requests

DATA_DIR = Path(__file__).parent

USDA_API_BASE = "https://api.nal.usda.gov/fdc/v1/foods/search"
USDA_API_KEY = os.environ.get("USDA_API_KEY")
if not USDA_API_KEY:
    raise SystemExit("Error: USDA_API_KEY environment variable is required. "
                     "Get a free key at https://fdc.nal.usda.gov/api-key-signup.html")


def fetch_foods(
    query: str,
    limit: int = 25,
) -> list[dict]:
    """Fetch branded food products from USDA FoodData Central API."""

    params = {
        "api_key": USDA_API_KEY,
        "query": query,
        "dataType": "Branded",
        "pageSize": min(limit, 50),
    }

    headers = {
        "User-Agent": "TeaLeaf-Benchmark/1.0 benchmark@tealeaf-project.dev",
    }

    all_foods = []
    total = None
    page = 1

    while True:
        print(f"  Fetching page {page}...")
        params["pageNumber"] = page
        resp = requests.get(USDA_API_BASE, params=params, headers=headers, timeout=60)
        resp.raise_for_status()
        data = resp.json()

        if total is None:
            total = data.get("totalHits", 0)
            print(f"  Total matching foods: {total}")

        foods = data.get("foods", [])
        if not foods:
            break

        all_foods.extend(foods)

        if len(all_foods) >= limit or len(all_foods) >= total:
            break

        page += 1

        # DEMO_KEY rate limit: 30 requests/hour
        print("  Waiting 3s for rate limit...")
        time.sleep(3)

    all_foods = all_foods[:limit]
    print(f"  Fetched {len(all_foods)} food products")
    return all_foods


def strip_empty(obj):
    """Recursively remove null values and empty lists/dicts from a JSON object."""
    if isinstance(obj, dict):
        return {
            k: strip_empty(v)
            for k, v in obj.items()
            if v is not None and v != [] and v != {}
        }
    elif isinstance(obj, list):
        return [strip_empty(item) for item in obj]
    return obj


def summarize(foods: list[dict]) -> dict:
    """Build summary statistics from raw USDA food records."""
    categories = {}
    brand_owners = {}
    total_nutrients = 0
    nutrient_names = set()

    for food in foods:
        # Categories
        cat = food.get("foodCategory", "Unknown")
        categories[cat] = categories.get(cat, 0) + 1

        # Brand owners
        owner = food.get("brandOwner", "Unknown")
        if owner:
            brand_owners[owner] = brand_owners.get(owner, 0) + 1

        # Nutrients
        nutrients = food.get("foodNutrients", [])
        total_nutrients += len(nutrients)
        for n in nutrients:
            name = n.get("nutrientName", "")
            if name:
                nutrient_names.add(name)

    return {
        "totalFoods": len(foods),
        "totalNutrientEntries": total_nutrients,
        "uniqueNutrients": len(nutrient_names),
        "avgNutrientsPerFood": round(total_nutrients / len(foods), 1) if foods else 0,
        "categoryDistribution": dict(sorted(categories.items(), key=lambda x: -x[1])[:10]),
        "topBrandOwners": dict(sorted(brand_owners.items(), key=lambda x: -x[1])[:10]),
    }


def main():
    parser = argparse.ArgumentParser(
        description="Fetch USDA FoodData Central data for benchmark"
    )
    parser.add_argument(
        "--query", default="breakfast cereal",
        help="Search query (default: 'breakfast cereal')"
    )
    parser.add_argument(
        "--limit", type=int, default=25,
        help="Maximum number of food products to fetch (default: 25)"
    )
    args = parser.parse_args()

    print(f"Fetching food products from USDA FoodData Central API...")
    print(f"  Query: {args.query}")
    print(f"  Limit: {args.limit}")

    # Fetch raw food products (preserving full USDA nesting)
    raw_foods = fetch_foods(args.query, args.limit)

    if not raw_foods:
        print("No food products found matching criteria.")
        return

    # Minimal cleanup: strip nulls and empty collections
    print("Cleaning records (stripping nulls/empties)...")
    foods = [strip_empty(food) for food in raw_foods]

    # Sort by description for consistent ordering
    foods.sort(key=lambda f: f.get("description", ""))

    # Build summary
    summary = summarize(foods)
    print(f"  Categories: {summary['categoryDistribution']}")
    print(f"  Total nutrient entries: {summary['totalNutrientEntries']}")
    print(f"  Avg nutrients per food: {summary['avgNutrientsPerFood']}")

    # Build output — wrap in metadata envelope, pass through USDA structure
    query_slug = args.query.replace(" ", "_")

    output = {
        "source": "USDA FoodData Central",
        "license": "US Government Public Domain",
        "url": "https://fdc.nal.usda.gov",
        "api": "FoodData Central API v1",
        "description": (
            f"{len(foods)} branded food products matching \"{args.query}\". "
            f"Original USDA structure preserved: foods[].foodNutrients[] with "
            f"nutrientId, nutrientName, unitName, value, derivation codes, and "
            f"percentDailyValue per entry."
        ),
        "query": {
            "searchTerm": args.query,
            "dataType": "Branded",
            "fetchedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        },
        "summary": summary,
        "foods": foods,
    }

    # Write output
    out_path = DATA_DIR / f"usda_branded_{query_slug}.json"
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(output, fh, indent=2, ensure_ascii=False)

    size = out_path.stat().st_size
    print(f"\nWritten: {out_path}")
    print(f"Size: {size:,} bytes ({size / 1024:.1f} KB)")
    print(f"Foods: {len(foods)}")
    print(f"Top brands: {list(summary['topBrandOwners'].keys())[:5]}")


if __name__ == "__main__":
    main()
