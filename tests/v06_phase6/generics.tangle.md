# Generic Type Inference Test

### ItemProcessor

#### process

* `items`: List of integers to double (List<Int>)
* `threshold`: Cutoff value (Int)

```@tangle
let doubled = List.map(items, fn(x) { x * 2 })
let filtered = List.filter(doubled, fn(x) { x > threshold })
return filtered
```

#### main

```@tangle
let numbers = [1, 2, 3]
let result = ItemProcessor.process(numbers, 2)
return result
```
