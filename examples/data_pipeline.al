// AgentLang Example: Multi-Stage Data Pipeline
//
// Demonstrates: realistic data processing with multiple operations,
// map construction, field extraction, and computation.
// Expected result: 150

SCHEMA Order => {
  product: Str,
  quantity: Int64,
  price: Int64
}

OPERATION create_order =>
  BODY {
    STORE order = { "product": "widget", "quantity": 5, "price": 30 }
    EMIT order
  }

OPERATION compute_total =>
  INPUT order: Map
  BODY {
    STORE qty = order.quantity
    STORE price = order.price
    STORE total = qty * price
    EMIT total
  }

PIPELINE Main => create_order -> compute_total
