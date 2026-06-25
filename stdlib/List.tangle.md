# List

### List (List)
* `items`: internal array (Array<T>)

#### 长度 (length)

```@tangle
return items.length
```

#### 映射 (map)
* `fn`: transform function (T -> U)

```@tangle
return List { items: items.map(fn) }
```

#### 过滤 (filter)
* `predicate`: filter function (T -> Bool)

```@tangle
return List { items: items.filter(predicate) }
```
