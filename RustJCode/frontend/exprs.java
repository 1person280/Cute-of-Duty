/*
 * 表达式子解析器（优先级分层）。
 *
 * 做什么：把表达式部分从 parser 里独立出来，按优先级自低到高递归下降：
 *   compare（== != < <= > >=）→ add（+ -）→ mul（* / %）→ unary（一元 -）→ atom（字面量/变量/括号）。
 *   独立成类，是为了让每个文件都守住「单文件 < 100 行」的上限，也让「语句」与「表达式」职责分明。
 *
 * 提供什么功能：
 *   - exprs(cursor cur, locals syms)：绑定游标与符号表。
 *   - parse()：解析一个完整表达式，返回其 AST 根节点（求值结果落在 EAX）。
 */
package frontend;

import ast.compute.*;
import ast.expr;
import ast.ident;
import ast.intlit;
import error.rustjerror;

public final class exprs {
    private final cursor cur;
    private final locals syms;

    public exprs(cursor cur, locals syms) {
        this.cur = cur;
        this.syms = syms;
    }

    public expr parse() {
        return compare();
    }

    private expr compare() {
        expr left = add();
        while (isCompare()) {
            String op = cur.take().text;
            left = compare(op, left, add());
        }
        return left;
    }

    private expr add() {
        expr left = mul();
        while (cur.peek("+") || cur.peek("-")) {
            String op = cur.take().text;
            expr right = mul();
            left = op.equals("+") ? new plus(left, right) : new minus(left, right);
        }
        return left;
    }

    private expr mul() {
        expr left = unary();
        while (cur.peek("*") || cur.peek("/") || cur.peek("%")) {
            String op = cur.take().text;
            expr right = unary();
            left = op.equals("*") ? new times(left, right)
                    : op.equals("/") ? new div(left, right) : new rem(left, right);
        }
        return left;
    }

    private expr unary() {
        if (cur.peek("-")) {
            cur.advance();
            return new neg(unary());
        }
        return atom();
    }

    private expr atom() {
        if (cur.peek("(")) {
            cur.advance();
            expr inner = compare();      // 括号内是完整表达式
            cur.expect(")");
            return inner;
        }
        token t = cur.take();
        if (t.type == token.kind.INT) return new intlit(Integer.parseInt(t.text));
        if (t.type == token.kind.IDENT) return new ident(syms.offsetOf(t.text, t.line));
        throw new rustjerror(t.line, "期望表达式，实际是 \"" + t.text + "\"");
    }

    private boolean isCompare() {
        return cur.peek("==") || cur.peek("!=") || cur.peek("<")
                || cur.peek("<=") || cur.peek(">") || cur.peek(">=");
    }

    private expr compare(String op, expr left, expr right) {
        if (op.equals("==")) return new eq(left, right);
        if (op.equals("!=")) return new ne(left, right);
        if (op.equals(">=")) return new ge(left, right);
        if (op.equals("<=")) return new le(left, right);
        if (op.equals(">")) return new gt(left, right);
        return new lt(left, right);
    }
}