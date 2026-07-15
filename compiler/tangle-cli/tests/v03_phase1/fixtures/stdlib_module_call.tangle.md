# StdlibModuleCallTest

[fmt](fmt)
[IO](IO)

## 测试

#### main

```@tangle
let line = fmt.println("hello", "world")
let content = IO.readFile("/tmp/test.txt")
return line
```
