# Option

### Option (Option)

Option represents a value that may or may not exist.

### Some (Some)
* `value`: the contained value (T)

#### 取值 (unwrap)

```@tangle
return value
```

### None (None)

#### 取值 (unwrap)

```@tangle
panic("Cannot unwrap None")
```
