# Tangle

Tangle is a Markdown-native programming language prototype.

## Track A1 frontend

```ts
import { compileModule } from "./src/index.js";

const mod = compileModule({
  file: "user.md",
  source: `# 用户中心

### User
* \`email\`: 邮箱 (String)

#### 激活 (activate)

\`\`\`@tangle
return this { is_active: true }
\`\`\`
`
});

console.log(mod.symbols);
```
