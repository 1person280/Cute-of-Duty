/*
 * return 语句（纯数据）。
 *
 * 做什么：表示 `return;` 或 `return 表达式;`，提前结束函数返回。
 * 提供什么功能：字段 value 为返回表达式，可缺省（为 null 时保留 EAX 现值）。
 */
package ast;

public final class returnstmt extends stmt {
    public final expr value;

    public returnstmt(expr value) {
        this.value = value;
    }
}