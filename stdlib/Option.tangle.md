# Option

### Option (Option)

Option represents a value that may or may not exist.

### Some (Some)
@export
* `value`: the contained value (T)

#### Some -> 取值 (unwrap)
@export

```@tangle
return value
```

### None (None)
@export

#### None -> 取值 (unwrap)

```@tangle
panic("Cannot unwrap None")
```
