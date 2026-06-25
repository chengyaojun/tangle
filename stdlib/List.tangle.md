# List

### List (List)
@export
* `items`: internal array (Array<T>)

#### 长度 (length)
@export

```@tangle
return items.length
```

#### 映射 (map)
@export
* `fn`: transform function (T -> U)

```@tangle
return List { items: items.map(fn) }
```

#### 过滤 (filter)
@export
* `predicate`: filter function (T -> Bool)

```@tangle
return List { items: items.filter(predicate) }
```
