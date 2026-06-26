# IoSystemDemo

Demonstrates IO, fmt, Env, and Path operations.

#### main

```@tangle
let data = IO.readFile("examples/collections.tangle.md")
fmt.println("File size: " + String.length(data))
let home = Env.get("HOME")
let joined = Path.join(home, "projects")
return joined
```
