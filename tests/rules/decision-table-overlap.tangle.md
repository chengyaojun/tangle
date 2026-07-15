# DecisionTableOverlapTest

决策表示例：含通配符和行重叠，验证 TANGLE_RULE_OVERLAP 诊断。

##### Rule: DecisionTableOverlap

| Income | Credit | Result |
|--------|--------|--------|
| high | - | approve |
| - | good | review |
| high | good | conflict |
| - | - | default |
