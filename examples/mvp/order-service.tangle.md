# Order Service
@entry

### Order
@export
* `id`: order ID (String)
* `amount`: order amount (Int)
* `status`: order status (String)

#### Order -> 创建订单 (create)
@export
* `id`: order ID (String)
* `amount`: amount (Int)

```@tangle
return Order { id: id, amount: amount, status: "created" }
```

#### Order -> 确认支付 (confirm)
@export
@error PayFailed
@error Timeout
* `order`: Order to confirm

```@tangle
if (order.amount <= 0) Err("PayFailed", "Invalid amount")?
if (order.amount > 10000) Err("Timeout", "Amount too large")?
return Ok(order with { status: "paid" })
```
