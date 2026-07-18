# ReturnConflictTest

### ConflictProcessor

#### process
* `input`: Input value (Int | String)

```@tangle
return match input {
  Int(x) => x,
  String(s) => s
}
```

#### main

```@tangle
return 0
```
