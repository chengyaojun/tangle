# CollectionsDemo

Demonstrates List, Map, Set, and Option collections.

#### main

```@tangle
let items = List { items: [1, 2, 3] }
let doubled = items.map(x -> x * 2)
let active = Map { entries: [("alice", true), ("bob", false)] }
let tags = Set { items: ["rust", "tangle"] }
let maybe = Option.Some(42)
return maybe.value
```
