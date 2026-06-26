# List

### List
* `items`: internal array (Array<T>)

#### length

```@tangle
return items.length
```

#### map
* `fn`: transform function (T -> U)

```@tangle
return this { items: items.map(fn) }
```

#### filter
* `predicate`: filter function (T -> Bool)

```@tangle
return this { items: items.filter(predicate) }
```
