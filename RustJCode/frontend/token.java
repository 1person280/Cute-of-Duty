/*
 * 词法单元（token）。
 *
 * 做什么：表示词法分析产出的一个最小文法单位，并记录其所在行号。
 *
 * 提供什么功能：
 *   - kind 枚举：区分标识符/关键字、整数字面量、`->`、单字符标点、结束符。
 *   - token(kind type, String text, int line)：保存类型、原文与行号，
 *     供语法分析匹配使用，并在报错时给出行定位。
 */
package frontend;

public final class token {
    public enum kind { IDENT, INT, ARROW, PUNCT, EOF }

    public final kind type;
    public final String text;
    public final int line;

    public token(kind type, String text, int line) {
        this.type = type;
        this.text = text;
        this.line = line;
    }
}