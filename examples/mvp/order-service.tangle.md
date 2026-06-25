# Order Service
@entry

### Order
* `id`: order ID (String)
* `amount`: order amount (Int)
* `status`: order status (String)

#### 创建订单 (create)
* `id`: order ID (String)
* `amount`: amount (Int)

```@tangle
return Order { id: id, amount: amount, status: "created" }
```

#### 确认支付 (confirm)
@error PayFailed
@error Timeout
* `order`: Order to confirm

```@tangle
if (order.amount <= 0) Err("PayFailed", "Invalid amount")?
if (order.amount > 10000) Err("Timeout", "Amount too large")?
return Ok(order { status: "paid" })
```
