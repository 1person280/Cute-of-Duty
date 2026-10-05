/*
 * 表达式子解析器（优先级分层）。
 *
 * 做什么：按优先级自低到高递归下降解析表达式：
 *   compare（== != < <= > >=）→ add（+ -）→ mul（* / %）→ unary（一元 -）→ atom（字面量/变量/括号）。
 *   独立成类，是为了让每个文件都守住「单文件 < 100 行」的上限，也让「语句」与「表达式」职责分明。
 *
 * 提供什么功能：
 *   - exprs(cursor cur, locals syms)：绑定游标与符号表。
 *   - parse()：解析一个完整表达式，返回其 AST 根节点（纯数据，发射交给后端）。
 */
package frontend;

import ast.binop;
import ast.expr;
import ast.ident;
import ast.intlit;
import ast.op;
import ast.unop;
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
            int kind = compareKind(cur.take().text);
            left = new binop(kind, left, add());
        }
        return left;
    }

    private expr add() {
        expr left = mul();
        while (cur.peek("+") || cur.peek("-")) {
            int kind = cur.take().text.equals("+") ? op.ADD : op.SUB;
            left = new binop(kind, left, mul());
        }
        return left;
    }

    private expr mul() {
        expr left = unary();
        while (cur.peek("*") || cur.peek("/") || cur.peek("%")) {
            String o = cur.take().text;
            int kind = o.equals("*") ? op.MUL : o.equals("/") ? op.DIV : op.REM;
            left = new binop(kind, left, unary());
        }
        return left;
    }

    private expr unary() {
        if (cur.peek("-")) {
            cur.advance();
            return new unop(op.NEG, unary());
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

    private int compareKind(String o) {
        if (o.equals("==")) return op.EQ;
        if (o.equals("!=")) return op.NE;
        if (o.equals("<=")) return op.LE;
        if (o.equals(">=")) return op.GE;
        if (o.equals("<")) return op.LT;
        return op.GT;
    }
}