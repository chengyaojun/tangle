# DecisionTreeTest

决策树示例：信用审批的 DNF（析取范式）规则。

##### Rule: CreditCheck

* Approve path
    * Income check: true
    * Credit check: true
    * Collateral: true
    * Action: approve
* Reject path
    * Action: reject
