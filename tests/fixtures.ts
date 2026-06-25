export const USER_MODULE = `# 用户中心

## 依赖

[Notify](./notify.md)

### User
* \`id\`: 用户 ID (Int)
* \`email\`: 邮箱 (String)

#### 激活 (activate)

\`\`\`@tangle
return this { is_active: true }
\`\`\`

##### 前置条件

###### 邮箱存在
> email must not be empty
`;

export const USER_MODULE_WITH_INTERFACE = `# User Service

### Notifyable (接口)

#### send (send)
* \`msg\`: message (String)

### User
* \`id\`: user ID (Int)
* \`email\`: email (String)

#### activate (activate)
* \`reason\`: activation reason (String)

\`\`\`@tangle
return this { is_active: true }
\`\`\`
`;
