# Tangle

Tangle is a Markdown-native programming language prototype.

## Track A1 frontend

```ts
import { compileModule } from "./src/index.js";

const mod = compileModule({
  file: "user.md",
  source: `# 用户中心

### User
@export
* \`email\`: 邮箱 (String)

#### User -> 激活 (activate)
@export

\`\`\`@tangle
return this with { is_active: true }
\`\`\`
`
});

console.log(mod.symbols);
```
