# JSON

### JSONUtils

#### 解析 (parse)
@export
* `input`: JSON string (String)

```@tangle
return JSON.parse(input)
```

#### 序列化 (stringify)
@export
* `value`: value to stringify (T)

```@tangle
return JSON.stringify(value)
```
