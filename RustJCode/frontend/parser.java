/*
 * 语法分析器（函数级）。
 *
 * 做什么：把 token 流解析为 ast.function——函数签名、函数体块（按序 let 语句 + 尾表达式）；
 *   变量偏移由 locals 在解析期解析并检查；表达式解析委托给 exprs。
 *
 * 提供什么功能：
 *   - parser(List<token> tokens)：绑定 token 流。
 *   - parse()：产出 ast.function（函数名 + 函数体块 + 栈帧字节数）。
 */
package frontend;

import ast.block;
import ast.expr;
import ast.function;
import ast.letstmt;
import ast.stmt;
import java.util.ArrayList;
import java.util.List;

public final class parser {
    private final cursor cur;

    public parser(List<token> tokens) {
        this.cur = new cursor(tokens);
    }

    public function parse() {
        cur.expect("fn");
        String name = cur.next(token.kind.IDENT, "函数名").text;
        cur.expect("(");
        cur.expect(")");
        cur.next(token.kind.ARROW, "->");
        cur.expect("i32");
        locals syms = new locals();
        block body = parseBlock(syms);
        return new function(name, body, syms.size() * 4);
    }

    private block parseBlock(locals syms) {
        cur.expect("{");
        exprs ex = new exprs(cur, syms);
        List<stmt> stmts = new ArrayList<>();
        while (cur.peek("let")) stmts.add(parseLet(syms, ex));
        expr tail = null;
        if (!cur.peek("}")) {
            if (cur.peek("return")) cur.advance();
            tail = ex.parse();
            if (cur.peek(";")) cur.advance();
        }
        cur.expect("}");
        return new block(stmts, tail);
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
}