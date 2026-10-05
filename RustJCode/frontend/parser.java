/*
 * 语法分析器（函数级）。
 *
 * 做什么：把 token 流解析为 ast.function——函数签名、按序解析 let 绑定、解析返回表达式；
 *         变量偏移由 locals 在解析期解析并检查；表达式解析委托给 exprs。
 *
 * 提供什么功能：
 *   - parser(List<token> tokens)：绑定 token 流。
 *   - parse()：产出 ast.function（函数名 + let 序列 + 返回表达式）。
 */
package frontend;

import ast.expr;
import ast.function;
import ast.letstmt;
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
        cur.expect("{");
        locals syms = new locals();
        exprs ex = new exprs(cur, syms);
        List<letstmt> lets = new ArrayList<>();
        while (cur.peek("let")) lets.add(parseLet(syms, ex));
        if (cur.peek("return")) cur.advance();
        expr body = ex.parse();
        if (cur.peek(";")) cur.advance();
        cur.expect("}");
        return new function(name, lets, body);
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