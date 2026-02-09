@struct billing_address (city: string, country: string, postal_code: string, state: string, street: string)
@struct customer (billing_address: billing_address, email: string, id: string, name: string, phone: string?, shipping_address: shipping_address?)
@struct customer_metric (avg_order_value: float, customer_id: string, total_orders: int, total_spent: float)
@struct item (line_number: int, notes: string?, options: []option, product_id: string, product_name: string, quantity: int, subtotal: float, unit_price: float, variant_sku: string)
@struct option (option_name: string, price_adjustment: float, selected_value: string)
@struct order (created_at: string, customer: []any, items: []item, notes: string?, order_id: string, order_number: string, payment: []any, shipping: []any?, status: string, totals: []float, updated_at: string)
@struct product (base_price: float, category: string, description: string, id: string, name: string, variants: []variant)
@struct shipping_address (city: string, country: string, postal_code: string, state: string, street: string)
@struct top_product (name: string, product_id: string, revenue: float, units_sold: int)
@struct variant (name: string, options: []object, price: float, sku: string, stock_quantity: int)

customers: @table customer [
  ((Seattle, USA, "98101", WA, "123 Main Street"), "alice.johnson@email.com", CUST-10001, "Alice Johnson", "+1-555-123-4567", (Seattle, USA, "98102", WA, "456 Work Ave, Suite 200")),
  ((Portland, USA, "97201", OR, "789 Oak Lane"), "bob.smith@company.com", CUST-10002, "Bob Smith", "+1-555-987-6543", ~),
  (("San Francisco", USA, "94102", CA, "321 Pine Road"), "carol.d@techcorp.io", CUST-10003, "Carol Davis", ~, ("Palo Alto", USA, "94301", CA, "100 Corporate Blvd"))
]
analytics: {total_orders: 10, average_order_value: 1470.59, customer_metrics: @table customer_metric [
  (877.88, CUST-10001, 4, 3511.51),
  (2316.24, CUST-10002, 3, 6948.73),
  (1415.24, CUST-10003, 3, 4245.71)
], top_products: @table top_product [
  ("Smart Watch Pro", PROD-003, 3582.0, 8),
  ("Mechanical Keyboard", PROD-004, 1243.0, 8),
  ("Premium Wireless Headphones", PROD-001, 3599.89, 11),
  ("Ergonomic Office Chair", PROD-002, 6281.0, 9)
], orders_by_status: [[delivered, 4], [shipped, 2], [processing, 1], [cancelled, 1], [partial_return, 1], [pending_payment, 1]], total_revenue: 14705.91, period: "2024-01-01 to 2024-03-10"}
metadata: {description: "Sample retail order data for analysis", generated_at: "2024-03-10T17:00:00Z", source: "Retail Order Management System", version: "1.0"}
orders: @table order [
  ("2024-01-15T10:28:00Z", [CUST-10001, "Alice Johnson", "alice.johnson@email.com", "+1-555-123-4567", ["123 Main Street", Seattle, WA, "98101", USA], ["456 Work Ave, Suite 200", Seattle, WA, "98102", USA]], [
    (1, ~, [
      (color, 0.0, Black),
      (warranty, 50.0, "5-Year Extended")
    ], PROD-001, "Premium Wireless Headphones", 1, 349.99, 349.99, SKU-001-BLK-EXT),
    (2, "Gift wrap requested", [
      (switches, 0.0, "Cherry MX Brown"),
      (layout, 0.0, "Full Size")
    ], PROD-004, "Mechanical Keyboard", 2, 318.0, 159.0, SKU-004-BRN),
    (3, ~, [
      (size, 30.0, "44mm"),
      (band, 70.0, "Brown Leather")
    ], PROD-003, "Smart Watch Pro", 1, 499.0, 499.0, SKU-003-44-LTH)
  ], "Leave at front door", ORD-2024-00001, WEB-1001, [credit_card, "4242", TXN-CC-78912345, 1236.61, completed, "2024-01-15T10:30:00Z"], [express, FedEx, FX123456789, "2024-01-17T14:00:00Z", 0.0], delivered, [1166.99, 0.1025, 119.62, 0.0, 50.0, 1236.61], "2024-01-17T15:30:00Z"),
  ("2024-01-20T08:45:00Z", [CUST-10002, "Bob Smith", "bob.smith@company.com", "+1-555-987-6543", ["789 Oak Lane", Portland, OR, "97201", USA], ~], [
    (1, "Corporate bulk order", [
      (color, 0.0, Graphite),
      (armrest, 50.0, "4D Adjustable"),
      (headrest, 100.0, "Adjustable Headrest")
    ], PROD-002, "Ergonomic Office Chair", 4, 2796.0, 699.0, SKU-002-GRY-FULL),
    (2, "Executive offices", [
      (color, 30.0, "Ocean Blue"),
      (armrest, 50.0, "4D Adjustable"),
      (headrest, 100.0, "Adjustable Headrest")
    ], PROD-002, "Ergonomic Office Chair", 2, 1458.0, 729.0, SKU-002-BLU-FULL)
  ], "Deliver to loading dock B. Call ahead.", ORD-2024-00002, WEB-1002, [purchase_order, ~, PO-2024-5678, 3978.6, pending, "2024-01-20T09:00:00Z"], [freight, "XPO Logistics", ~, "2024-02-01T00:00:00Z", 150.0], processing, [4254.0, 0.0, 0.0, 150.0, 425.4, 3978.6], "2024-01-20T09:00:00Z"),
  ("2024-01-22T16:42:00Z", [CUST-10003, "Carol Davis", "carol.d@techcorp.io", ~, ["321 Pine Road", "San Francisco", CA, "94102", USA], ["100 Corporate Blvd", "Palo Alto", CA, "94301", USA]], [
    (1, ~, [
      (size, 30.0, "44mm"),
      (band, 170.0, "Stainless Steel")
    ], PROD-003, "Smart Watch Pro", 1, 599.0, 599.0, SKU-003-44-SS)
  ], ~, ORD-2024-00003, WEB-1003, [apple_pay, ~, AP-9876543210, 676.41, completed, "2024-01-22T16:45:00Z"], [next_day, UPS, "1Z999AA10123456784", "2024-01-23T18:00:00Z", 25.0], shipped, [599.0, 0.0875, 52.41, 25.0, 0.0, 676.41], "2024-01-23T08:15:00Z"),
  ("2024-01-25T11:18:00Z", [CUST-10001, "Alice Johnson", "alice.johnson@email.com", "+1-555-123-4567", ["123 Main Street", Seattle, WA, "98101", USA], ~], [
    (1, "Birthday gift", [
      (color, 0.0, White),
      (warranty, 0.0, "2-Year Standard")
    ], PROD-001, "Premium Wireless Headphones", 1, 299.99, 299.99, SKU-001-WHT),
    (2, ~, [
      (switches, 0.0, "Cherry MX Red"),
      (layout, 0.0, "Full Size")
    ], PROD-004, "Mechanical Keyboard", 1, 159.0, 159.0, SKU-004-RED)
  ], ~, ORD-2024-00004, WEB-1004, [credit_card, "1234", TXN-CC-45678901, 506.04, completed, "2024-01-25T11:20:00Z"], [standard, USPS, "9400111899223033005", "2024-01-30T12:00:00Z", 0.0], delivered, [458.99, 0.1025, 47.05, 0.0, 0.0, 506.04], "2024-01-30T14:22:00Z"),
  ("2024-02-01T13:55:00Z", [CUST-10002, "Bob Smith", "bob.smith@company.com", "+1-555-987-6543", ["789 Oak Lane", Portland, OR, "97201", USA], ~], [
    (1, ~, [
      (switches, 0.0, "Cherry MX Brown"),
      (layout, 0.0, "Full Size")
    ], PROD-004, "Mechanical Keyboard", 1, 159.0, 159.0, SKU-004-BRN),
    (2, ~, [
      (switches, 0.0, "Cherry MX Blue"),
      (layout, 0.0, "Full Size")
    ], PROD-004, "Mechanical Keyboard", 1, 159.0, 159.0, SKU-004-BLU),
    (3, "For home office", [
      (switches, 0.0, "Cherry MX Brown"),
      (layout, -10.0, Tenkeyless)
    ], PROD-004, "Mechanical Keyboard", 2, 298.0, 149.0, SKU-004-BRN-TKL)
  ], ~, ORD-2024-00005, WEB-1005, [paypal, ~, PP-ABC123DEF456, 585.2, completed, "2024-02-01T14:00:00Z"], [standard, USPS, "9400111899223033006", "2024-02-05T10:00:00Z", 0.0], delivered, [616.0, 0.0, 0.0, 0.0, 30.8, 585.2], "2024-02-05T11:30:00Z"),
  ("2024-02-10T09:25:00Z", [CUST-10003, "Carol Davis", "carol.d@techcorp.io", ~, ["321 Pine Road", "San Francisco", CA, "94102", USA], ~], [
    (1, "Employee gifts", [
      (size, 0.0, "40mm"),
      (band, 0.0, "Black Sport")
    ], PROD-003, "Smart Watch Pro", 3, 1197.0, 399.0, SKU-003-40-BLK),
    (2, ~, [
      (size, 30.0, "44mm"),
      (band, 0.0, "Black Sport")
    ], PROD-003, "Smart Watch Pro", 2, 858.0, 429.0, SKU-003-44-BLK)
  ], "Corporate account - tax exempt certificate on file", ORD-2024-00006, WEB-1006, [credit_card, "5678", TXN-CC-11223344, 2132.06, completed, "2024-02-10T09:30:00Z"], [express, FedEx, FX987654321, "2024-02-12T16:00:00Z", 0.0], shipped, [2055.0, 0.0875, 179.81, 0.0, 102.75, 2132.06], "2024-02-11T08:00:00Z"),
  ("2024-02-15T09:45:00Z", [CUST-10001, "Alice Johnson", "alice.johnson@email.com", "+1-555-123-4567", ["123 Main Street", Seattle, WA, "98101", USA], ~], [
    (1, ~, [
      (color, 0.0, Graphite),
      (armrest, 50.0, "4D Adjustable"),
      (headrest, 0.0, None)
    ], PROD-002, "Ergonomic Office Chair", 1, 599.0, 599.0, SKU-002-GRY-ADJ)
  ], "Customer requested cancellation - found better price elsewhere", ORD-2024-00007, WEB-1007, [credit_card, "4242", TXN-CC-99887766, 735.4, refunded, "2024-02-15T10:00:00Z"], ~, cancelled, [599.0, 0.1025, 61.4, 75.0, 0.0, 735.4], "2024-02-15T14:30:00Z"),
  ("2024-02-20T10:30:00Z", [CUST-10002, "Bob Smith", "bob.smith@company.com", "+1-555-987-6543", ["789 Oak Lane", Portland, OR, "97201", USA], ~], [
    (1, "Team equipment", [
      (color, 0.0, Black),
      (warranty, 50.0, "5-Year Extended")
    ], PROD-001, "Premium Wireless Headphones", 5, 1749.95, 349.99, SKU-001-BLK-EXT),
    (2, ~, [
      (color, 0.0, White),
      (warranty, 0.0, "2-Year Standard")
    ], PROD-001, "Premium Wireless Headphones", 3, 899.97, 299.99, SKU-001-WHT)
  ], "Bulk corporate order - 10% volume discount applied", ORD-2024-00008, WEB-1008, [wire_transfer, ~, WT-2024-789012, 2384.93, completed, "2024-02-20T11:00:00Z"], [standard, FedEx, FX111222333, "2024-02-25T12:00:00Z", 0.0], delivered, [2649.92, 0.0, 0.0, 0.0, 264.99, 2384.93], "2024-02-25T13:45:00Z"),
  ("2024-02-25T14:50:00Z", [CUST-10003, "Carol Davis", "carol.d@techcorp.io", ~, ["321 Pine Road", "San Francisco", CA, "94102", USA], ["100 Corporate Blvd", "Palo Alto", CA, "94301", USA]], [
    (1, "RETURNED - wrong color", [
      (color, 30.0, "Ocean Blue"),
      (armrest, 50.0, "4D Adjustable"),
      (headrest, 100.0, "Adjustable Headrest")
    ], PROD-002, "Ergonomic Office Chair", 1, 729.0, 729.0, SKU-002-BLU-FULL),
    (2, ~, [
      (switches, 0.0, "Cherry MX Brown"),
      (layout, 0.0, "Full Size")
    ], PROD-004, "Mechanical Keyboard", 1, 159.0, 159.0, SKU-004-BRN),
    (3, ~, [
      (size, 30.0, "44mm"),
      (band, 0.0, "Black Sport")
    ], PROD-003, "Smart Watch Pro", 1, 429.0, 429.0, SKU-003-44-BLK)
  ], "Chair returned 2024-03-05. Refund processed: $729.00 + tax", ORD-2024-00009, WEB-1009, [credit_card, "9999", TXN-CC-55443322, 1432.24, partial_refund, "2024-02-25T15:00:00Z"], [standard, UPS, "1Z999AA10123456785", "2024-02-28T14:00:00Z", 0.0], partial_return, [1317.0, 0.0875, 115.24, 0.0, 0.0, 1432.24], "2024-03-06T10:00:00Z"),
  ("2024-03-10T16:25:00Z", [CUST-10001, "Alice Johnson", "alice.johnson@email.com", "+1-555-123-4567", ["123 Main Street", Seattle, WA, "98101", USA], ["456 Work Ave, Suite 200", Seattle, WA, "98102", USA]], [
    (1, ~, [
      (color, 0.0, Graphite),
      (armrest, 50.0, "4D Adjustable"),
      (headrest, 100.0, "Adjustable Headrest")
    ], PROD-002, "Ergonomic Office Chair", 1, 699.0, 699.0, SKU-002-GRY-FULL),
    (2, ~, [
      (color, 0.0, Black),
      (warranty, 0.0, "2-Year Standard")
    ], PROD-001, "Premium Wireless Headphones", 1, 299.99, 299.99, SKU-001-BLK),
    (3, "Engraving: 'AD 2024'", [
      (size, 30.0, "44mm"),
      (band, 70.0, "Brown Leather")
    ], PROD-003, "Smart Watch Pro", 1, 499.0, 499.0, SKU-003-44-LTH),
    (4, ~, [
      (switches, 0.0, "Cherry MX Brown"),
      (layout, -10.0, Tenkeyless)
    ], PROD-004, "Mechanical Keyboard", 1, 149.0, 149.0, SKU-004-BRN-TKL)
  ], "Payment processing - awaiting card verification", ORD-2024-00010, WEB-1010, [credit_card, ~, ~, 0.0, awaiting, "2024-03-10T16:30:00Z"], ~, pending_payment, [1646.99, 0.1025, 168.82, 0.0, 82.35, 1733.46], "2024-03-10T16:30:00Z")
]
products: @table product [
  (299.99, "Electronics/Audio", "High-fidelity wireless headphones with active noise cancellation", PROD-001, "Premium Wireless Headphones", [
    ("Midnight Black", [{price_modifier: 0.0, value: Black, name: color}, {price_modifier: 0.0, value: "2-Year Standard", name: warranty}], 299.99, SKU-001-BLK, 150),
    ("Pearl White", [{name: color, price_modifier: 0.0, value: White}, {price_modifier: 0.0, value: "2-Year Standard", name: warranty}], 299.99, SKU-001-WHT, 85),
    ("Midnight Black + Extended Warranty", [{price_modifier: 0.0, name: color, value: Black}, {price_modifier: 50.0, name: warranty, value: "5-Year Extended"}], 349.99, SKU-001-BLK-EXT, 45)
  ]),
  (549.0, "Furniture/Office", "Adjustable lumbar support, breathable mesh, 10-year warranty", PROD-002, "Ergonomic Office Chair", [
    ("Graphite - Standard Arms", [{price_modifier: 0.0, value: Graphite, name: color}, {name: armrest, price_modifier: 0.0, value: "Standard Fixed"}, {price_modifier: 0.0, name: headrest, value: None}], 549.0, SKU-002-GRY-STD, 32),
    ("Graphite - Adjustable Arms", [{value: Graphite, name: color, price_modifier: 0.0}, {value: "4D Adjustable", name: armrest, price_modifier: 50.0}, {value: None, name: headrest, price_modifier: 0.0}], 599.0, SKU-002-GRY-ADJ, 28),
    ("Graphite - Full Options", [{value: Graphite, price_modifier: 0.0, name: color}, {price_modifier: 50.0, value: "4D Adjustable", name: armrest}, {name: headrest, price_modifier: 100.0, value: "Adjustable Headrest"}], 699.0, SKU-002-GRY-FULL, 15),
    ("Ocean Blue - Full Options", [{name: color, price_modifier: 30.0, value: "Ocean Blue"}, {name: armrest, price_modifier: 50.0, value: "4D Adjustable"}, {name: headrest, price_modifier: 100.0, value: "Adjustable Headrest"}], 729.0, SKU-002-BLU-FULL, 8)
  ]),
  (399.0, "Electronics/Wearables", "Health monitoring, GPS, 5-day battery life", PROD-003, "Smart Watch Pro", [
    ("40mm Black Sport Band", [{name: size, price_modifier: 0.0, value: "40mm"}, {name: band, price_modifier: 0.0, value: "Black Sport"}], 399.0, SKU-003-40-BLK, 200),
    ("44mm Black Sport Band", [{name: size, price_modifier: 30.0, value: "44mm"}, {price_modifier: 0.0, name: band, value: "Black Sport"}], 429.0, SKU-003-44-BLK, 180),
    ("44mm Leather Band", [{price_modifier: 30.0, name: size, value: "44mm"}, {name: band, value: "Brown Leather", price_modifier: 70.0}], 499.0, SKU-003-44-LTH, 50),
    ("44mm Stainless Steel Band", [{price_modifier: 30.0, value: "44mm", name: size}, {value: "Stainless Steel", name: band, price_modifier: 170.0}], 599.0, SKU-003-44-SS, 25)
  ]),
  (159.0, "Electronics/Peripherals", "Hot-swappable switches, RGB backlight, aluminum frame", PROD-004, "Mechanical Keyboard", [
    ("Brown Switches - Quiet", [{name: switches, price_modifier: 0.0, value: "Cherry MX Brown"}, {value: "Full Size", name: layout, price_modifier: 0.0}], 159.0, SKU-004-BRN, 75),
    ("Red Switches - Linear", [{price_modifier: 0.0, value: "Cherry MX Red", name: switches}, {name: layout, price_modifier: 0.0, value: "Full Size"}], 159.0, SKU-004-RED, 60),
    ("Blue Switches - Clicky", [{value: "Cherry MX Blue", name: switches, price_modifier: 0.0}, {price_modifier: 0.0, value: "Full Size", name: layout}], 159.0, SKU-004-BLU, 45),
    ("Brown Switches - TKL", [{name: switches, price_modifier: 0.0, value: "Cherry MX Brown"}, {price_modifier: -10.0, value: Tenkeyless, name: layout}], 149.0, SKU-004-BRN-TKL, 40)
  ])
]
