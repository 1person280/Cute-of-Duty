/*
 * 词法分析器。
 *
 * 做什么：把 RustJ 极小子集源码切分为 token 流，并跳过空白与 `//` 行注释。
 *
 * 提供什么功能：
 *   - lexer(String src)：绑定待分析的源码字符串。
 *   - tokenize()：返回 token 列表，末尾必定附加一个 EOF 单元。
 *   - 识别范围：标识符/关键字（含 let）、整数字面量、`->`，
 *     以及单字符标点（供 + - * ( ) ; = 等使用）。
 */
package frontend;

import java.util.ArrayList;
import java.util.List;

public final class lexer {
    private final String src;
    private int pos = 0;
    private int line = 1;

    public lexer(String src) {
        this.src = src;
    }

    public List<token> tokenize() {
        List<token> out = new ArrayList<>();
        while (true) {
            skipTrivia();
            if (pos >= src.length()) {
                out.add(new token(token.kind.EOF, "", line));
                return out;
            }
            char c = src.charAt(pos);
            if (Character.isLetter(c) || c == '_') {
                out.add(readIdent());
            } else if (Character.isDigit(c)) {
                out.add(readInt());
            } else if (c == '-' && peek(1) == '>') {
                pos += 2;
                out.add(new token(token.kind.ARROW, "->", line));
            } else if ((c == '=' || c == '!' || c == '<' || c == '>') && peek(1) == '=') {
                pos += 2;
                out.add(new token(token.kind.PUNCT, src.substring(pos - 2, pos), line));
            } else {
                pos++;
                out.add(new token(token.kind.PUNCT, String.valueOf(c), line));
            }
        }
    }

    private void skipTrivia() {
        while (pos < src.length()) {
            char c = src.charAt(pos);
            if (c == '\n') { line++; pos++; }
            else if (Character.isWhitespace(c)) { pos++; }
            else if (c == '/' && peek(1) == '/') {
                while (pos < src.length() && src.charAt(pos) != '\n') pos++;
            } else return;
        }
    }

    private char peek(int ahead) {
        int i = pos + ahead;
        return i < src.length() ? src.charAt(i) : '\0';
    }

    private token readIdent() {
        int start = pos;
        while (pos < src.length()
                && (Character.isLetterOrDigit(src.charAt(pos)) || src.charAt(pos) == '_')) pos++;
        return new token(token.kind.IDENT, src.substring(start, pos), line);
    }

    private token readInt() {
        int start = pos;
        while (pos < src.length() && Character.isDigit(src.charAt(pos))) pos++;
        return new token(token.kind.INT, src.substring(start, pos), line);
    }
}