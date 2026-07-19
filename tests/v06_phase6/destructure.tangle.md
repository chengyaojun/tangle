# DestructureTest

### Item
* `name`: item name (String)
* `price`: item price (Int)

#### make
* `name`: item name (String)
* `price`: item price (Int)

```@tangle
return Item { name: name, price: price }
```

### DestructureProcessor

#### process
* `opt`: Optional value (Option<Item>)

```@tangle
let Some(item) = opt else {
  return Item { name: "default", price: 0 }
}
let { name, price } = item
return item
```

#### main

```@tangle
return 0
```
