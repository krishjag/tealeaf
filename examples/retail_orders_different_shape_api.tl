@struct shipping (name: string, street: string, city: string, state: string, zip: string, country: string)
@struct price (base_price: float, discount_pct: float?, currency: string, tax_rate: float)
@struct stock (warehouse: string, qty_available: int, reorder_level: int, backordered: bool)
@struct product (id: string, name: string, sku: string, category: string, brand: string, price: price, stock: []stock)
@struct item (line_number: int, product: product, quantity: int, unit_price: float, line_total: float, notes: string?)
@struct order (order_id: string, status: string, placed_at: string, shipped_at: string?, customer_id: string, customer_email: string, shipping: shipping, items: []item, subtotal: float, tax: float, shipping_cost: float, total: float, payment_method: string)

orders: @table order [
  (ORD-2025-00847, shipped, "2025-06-14T09:23:17Z", "2025-06-15T14:02:00Z", CUST-30291, "marcus.chen@gmail.com", ("Marcus Chen", "4521 Olive Branch Ln", Portland, OR, "97201", US), [
    (1, (PROD-7841, "Sony WH-1000XM5 Wireless Headphones", SNY-WH1000XM5-BLK, "Electronics/Audio", Sony, (399.99, 15.0, USD, 8.25), [
      (US-WEST-01, 342, 50, false),
      (US-EAST-02, 189, 50, false)
    ]), 1, 339.99, 339.99, "Gift wrap requested"),
    (2, (PROD-2156, "Anker USB-C to Lightning Cable 6ft", ANK-USBC-LTN-6FT, "Electronics/Accessories", Anker, (15.99, ~, USD, 8.25), [
      (US-WEST-01, 2841, 500, false)
    ]), 2, 15.99, 31.98, ~)
  ], 371.97, 30.69, 0.0, 402.66, visa_ending_4242),
  (ORD-2025-00848, processing, "2025-06-14T11:47:53Z", ~, CUST-18774, "sofia.r@outlook.com", ("Sofia Ramirez", "88 Beacon St Apt 3F", Boston, MA, "02108", US), [
    (1, (PROD-9034, "Patagonia Better Sweater Jacket - Navy L", PAT-BSWEATER-NVY-L, "Apparel/Outerwear", Patagonia, (149.0, 20.0, USD, 9.5), [
      (US-WEST-01, 23, 30, false),
      (US-EAST-02, 0, 30, true)
    ]), 1, 119.2, 119.2, ~),
    (2, (PROD-5512, "Chemex 8-Cup Pour-Over Coffee Maker", CHX-POUROVER-8CUP, "Home/Kitchen", Chemex, (44.95, ~, USD, 8.25), [
      (US-EAST-02, 567, 100, false)
    ]), 1, 44.95, 44.95, "Include Chemex filters sample pack"),
    (3, (PROD-2156, "Anker USB-C to Lightning Cable 6ft", ANK-USBC-LTN-6FT, "Electronics/Accessories", Anker, (15.99, ~, USD, 8.25), [
      (US-WEST-01, 2841, 500, false)
    ]), 3, 15.99, 47.97, ~)
  ], 212.12, 18.9, 7.99, 239.01, "paypal_sofia.r@outlook.com")
]
