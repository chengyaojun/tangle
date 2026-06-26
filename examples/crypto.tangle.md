# CryptoDemo

Demonstrates Crypto and Encoding operations.

#### main

```@tangle
let data = "hello tangle"
let hash = Crypto.sha256(data)
let encoded = Encoding.hex_encode(hash)
return encoded
```
