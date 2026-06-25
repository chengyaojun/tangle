# Map

### Map (Map)
* `entries`: key-value pairs (Array<{key: K, value: V}>)

#### 获取 (get)
* `key`: lookup key (K)

```@tangle
match entries.find(e -> e.key == key) {
  Some(entry) -> entry.value
  None -> panic("Key not found")
}
```
