/*
 * 语句与块解析。
 *
 * 做什么：把「块 := '{' 语句* 尾表达式? '}'」及其中的语句形态（let / 赋值 / if / while / return）
 *   从 parser 里独立出来，使每个文件都守住「单文件 < 100 行」的上限。
 *   块可嵌套，因此 if/while 的体也复用本类递归解析。
 *
 * 提供什么功能：
 *   - stmts(cursor cur)：绑定游标。
 *   - parse(locals syms)：解析一个块，返回 ast.block（语句列表 + 可选尾表达式）。
 */
package frontend;

import ast.assignstmt;
import ast.block;
import ast.expr;
import ast.ifstmt;
import ast.letstmt;
import ast.returnstmt;
import ast.stmt;
import ast.whilestmt;
import java.util.ArrayList;
import java.util.List;

public final class stmts {
    private final cursor cur;

    public stmts(cursor cur) {
        this.cur = cur;
    }

    public block parse(locals syms) {
        cur.expect("{");
        exprs ex = new exprs(cur, syms);
        List<stmt> list = new ArrayList<>();
        while (true) {
            if (cur.peek("let")) list.add(parseLet(syms, ex));
            else if (cur.peek("if")) list.add(parseIf(syms, ex));
            else if (cur.peek("while")) list.add(parseWhile(syms, ex));
            else if (cur.peek("return")) list.add(parseReturn(ex));
            else if (cur.isIdent() && cur.ahead(1).equals("=")) list.add(parseAssign(syms, ex));
            else break;
        }
        expr tail = cur.peek("}") ? null : ex.parse();
        if (cur.peek(";")) cur.advance();
        cur.expect("}");
        return new block(list, tail);
    }

    private letstmt parseLet(locals syms, exprs ex) {
        cur.expect("let");
        token nm = cur.next(token.kind.IDENT, "变量名");
        int offset = syms.declare(nm.text, nm.line);
        cur.expect("=");
        expr init = ex.parse();
        if (cur.peek(";")) cur.advance();
        return new letstmt(offset, init);
    }

    private assignstmt parseAssign(locals syms, exprs ex) {
        token nm = cur.take();
        int offset = syms.offsetOf(nm.text, nm.line);
        cur.expect("=");
        expr value = ex.parse();
        if (cur.peek(";")) cur.advance();
        return new assignstmt(offset, value);
    }

    private ifstmt parseIf(locals syms, exprs ex) {
        cur.expect("if");
        expr cond = ex.parse();
        block then = parse(syms);
        block els = null;
        if (cur.peek("else")) {
            cur.advance();
            els = parse(syms);
        }
        return new ifstmt(cond, then, els);
    }

    private whilestmt parseWhile(locals syms, exprs ex) {
        cur.expect("while");
        expr cond = ex.parse();
        return new whilestmt(cond, parse(syms));
    }

    private returnstmt parseReturn(exprs ex) {
        cur.expect("return");
        expr value = cur.peek(";") ? null : ex.parse();
        if (cur.peek(";")) cur.advance();
        return new returnstmt(value);
    }
}