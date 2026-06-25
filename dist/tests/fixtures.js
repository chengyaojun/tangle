export const USER_MODULE = `# 用户中心

## 依赖

[Notify](./notify.md)

### User
@export
* \`id\`: 用户 ID (Int)
* \`email\`: 邮箱 (String)

#### User -> 激活 (activate)
@export

\`\`\`@tangle
return this with { is_active: true }
\`\`\`

##### 前置条件

###### 邮箱存在
> email must not be empty
`;
export const USER_MODULE_WITH_INTERFACE = `# User Service

### Notifyable (接口)

#### Notifyable -> send (send)
* \`msg\`: message (String)

### User
@export
* \`id\`: user ID (Int)
* \`email\`: email (String)

#### User -> activate (activate)
@export
* \`reason\`: activation reason (String)

\`\`\`@tangle
return this with { is_active: true }
\`\`\`
`;
