# OrderService

### Order
* `id`: order ID (String)
* `amount`: order amount (Int)
* `status`: order status (String)

#### create
* `id`: order ID (String)
* `amount`: amount (Int)

```@tangle
return Order { id: id, amount: amount, status: "created" }
```

#### confirm
##### Error: PayFailed
##### Error: Timeout
* `order`: Order to confirm

```@tangle
if (order.amount <= 0) Err("PayFailed", "Invalid amount")?
if (order.amount > 10000) Err("Timeout", "Amount too large")?
return Ok(order { status: "paid" })
```

#### main
* `config`: (Config)

```@tangle
let order = Order.create("ord-1", 100)?
return order { status: "confirmed" }
```
